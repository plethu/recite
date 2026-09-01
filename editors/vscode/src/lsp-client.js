import { EventEmitter } from "node:events";
import { spawn } from "node:child_process";
import { encodeMessage, LspFrameParser } from "./lsp-protocol.js";

const SHUTDOWN_TIMEOUT_MS = 1000;

export class ReciteLanguageClient extends EventEmitter {
  constructor(options) {
    super();
    this.command = options.command;
    this.args = [...options.args];
    this.cwd = options.cwd;
    this.environment = options.environment;
    this.spawnProcess = options.spawnProcess ?? spawn;
    this.child = undefined;
    this.state = "idle";
    this.nextRequestId = 1;
    this.pending = new Map();
    this.parser = new LspFrameParser((message) => this.receive(message));
  }

  get status() {
    return this.state;
  }

  async start(initializeParams) {
    if (this.state !== "idle") {
      throw new Error(`cannot start language server from ${this.state} state`);
    }
    this.state = "starting";
    const environment = { ...process.env, ...(this.environment ?? {}) };
    try {
      this.child = this.spawnProcess(this.command, this.args, {
        cwd: this.cwd,
        env: environment,
        shell: false,
        stdio: ["pipe", "pipe", "pipe"]
      });
    } catch (error) {
      this.state = "stopped";
      throw error;
    }
    this.exited = false;
    this.child.stdout.on("data", (chunk) => this.read(chunk));
    this.child.stderr.on("data", (chunk) => this.emit("stderr", chunk.toString("utf8")));
    this.child.on("error", (error) => this.fail(error));
    this.child.on("exit", (code, signal) => {
      this.exited = true;
      if (this.state !== "stopping" && this.state !== "stopped") {
        this.fail(new Error(`recite-lsp exited (${code ?? "unknown"}, ${signal ?? "no signal"})`));
      }
      this.state = "stopped";
      this.emit("exit", { code, signal });
    });

    try {
      const result = await this.request("initialize", initializeParams);
      this.notify("initialized", {});
      this.state = "running";
      return result;
    } catch (error) {
      await this.stop();
      throw error;
    }
  }

  read(chunk) {
    try {
      this.parser.push(chunk);
    } catch (error) {
      this.fail(error);
      this.child?.kill();
    }
  }

  request(method, params) {
    if (!this.child?.stdin.writable || this.state === "stopped") {
      return Promise.reject(new Error("recite-lsp is not running"));
    }
    const id = this.nextRequestId++;
    const message = { jsonrpc: "2.0", id, method };
    if (params !== undefined) message.params = params;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      try {
        this.child.stdin.write(encodeMessage(message));
      } catch (error) {
        this.pending.delete(id);
        reject(error);
      }
    });
  }

  notify(method, params) {
    if (!this.child?.stdin.writable || this.state === "stopped") return false;
    const message = { jsonrpc: "2.0", method };
    if (params !== undefined) message.params = params;
    this.child.stdin.write(encodeMessage(message));
    return true;
  }

  receive(message) {
    if (message && Object.hasOwn(message, "id") &&
        (Object.hasOwn(message, "result") || Object.hasOwn(message, "error"))) {
      const pending = this.pending.get(message.id);
      if (!pending) return;
      this.pending.delete(message.id);
      if (message.error) {
        const error = new Error(message.error.message ?? "LSP request failed");
        error.code = message.error.code;
        error.data = message.error.data;
        pending.reject(error);
      } else {
        pending.resolve(message.result);
      }
      return;
    }

    if (message && Object.hasOwn(message, "id") && message.method) {
      this.handleServerRequest(message);
      return;
    }
    if (message?.method) this.emit("notification", message.method, message.params);
  }

  handleServerRequest(message) {
    if (message.method === "client/registerCapability" ||
        message.method === "window/workDoneProgress/create") {
      this.respond(message.id, {});
      return;
    }
    if (message.method === "workspace/configuration") {
      this.respond(message.id, []);
      return;
    }
    this.respond(message.id, undefined, { code: -32601, message: `unsupported client method: ${message.method}` });
  }

  respond(id, result, error) {
    if (!this.child?.stdin.writable || this.state === "stopped") return;
    const message = { jsonrpc: "2.0", id };
    if (error) message.error = error;
    else message.result = result;
    this.child.stdin.write(encodeMessage(message));
  }

  async stop() {
    if (!this.child || this.state === "stopped" || this.state === "stopping") return;
    this.state = "stopping";
    const child = this.child;
    const exited = new Promise((resolve) => child.once("exit", resolve));
    if (child.stdin.writable) {
      try {
        await Promise.race([
          this.request("shutdown", undefined),
          timeout(SHUTDOWN_TIMEOUT_MS)
        ]);
      } catch {
        // A broken or already-exiting server still needs the exit notification
        // when its transport is writable; shutdown is best effort at teardown.
      }
      this.notify("exit", undefined);
      child.stdin.end();
    }
    await Promise.race([exited, timeout(SHUTDOWN_TIMEOUT_MS)]);
    if (!this.exited && !child.killed) child.kill();
    this.rejectPending(new Error("recite-lsp stopped"));
    this.state = "stopped";
  }

  fail(error) {
    this.rejectPending(error);
    this.emit("serverError", error);
  }

  rejectPending(error) {
    for (const pending of this.pending.values()) pending.reject(error);
    this.pending.clear();
  }
}

function timeout(milliseconds) {
  return new Promise((resolve) => {
    const timer = setTimeout(resolve, milliseconds);
    timer.unref?.();
  });
}
