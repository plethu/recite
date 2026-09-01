import { EventEmitter } from "node:events";
import { spawn } from "node:child_process";
import { encodeMessage, LspFrameParser } from "./lsp-protocol.js";
import {
  asClientFailure,
  ClientFailure,
  ClientFailureKind
} from "./client-failure.js";

const SHUTDOWN_TIMEOUT_MS = 1000;

export class ReciteLanguageClient extends EventEmitter {
  constructor(options) {
    super();
    this.command = options.command;
    this.args = [...options.args];
    this.cwd = options.cwd;
    this.environment = options.environment;
    this.spawnProcess = options.spawnProcess ?? spawn;
    this.onRegisterCapability = options.onRegisterCapability;
    this.onUnregisterCapability = options.onUnregisterCapability;
    this.child = undefined;
    this.state = "idle";
    this.nextRequestId = 1;
    this.pending = new Map();
    this.queuedNotifications = [];
    this.transportClosed = false;
    this.failure = undefined;
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
      throw asClientFailure(ClientFailureKind.Lifecycle, error);
    }
    this.exited = false;
    this.transportClosed = false;
    this.failure = undefined;
    this.child.stdout.on("data", (chunk) => this.read(chunk));
    this.child.stderr.on("data", (chunk) => this.emit("stderr", chunk.toString("utf8")));
    // A Writable emits EPIPE asynchronously. Attach this before the first
    // initialize write so a child that exits immediately cannot surface an
    // uncaught exception or leave a pending request hanging.
    this.child.stdin.on("error", (error) => this.handleFailure(
      ClientFailureKind.Transport, error
    ));
    this.child.on("error", (error) => this.handleFailure(
      ClientFailureKind.Lifecycle, error
    ));
    this.child.on("exit", (code, signal) => {
      this.exited = true;
      if (this.state !== "stopping" && this.state !== "stopped") {
        this.rejectPending(new ClientFailure(
          ClientFailureKind.Lifecycle,
          exitDetail(code, signal)
        ));
      }
      this.state = "stopped";
      this.emit("exit", { code, signal });
    });

    try {
      const result = await this.request("initialize", initializeParams);
      this.state = "running";
      this.notify("initialized", {}, { queue: false });
      this.flushNotifications();
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
      this.handleFailure(ClientFailureKind.Protocol, error);
    }
  }

  request(method, params) {
    if (!this.child?.stdin.writable || this.state === "stopped" || this.transportClosed) {
      return Promise.reject(new ClientFailure(ClientFailureKind.Lifecycle));
    }
    const id = this.nextRequestId++;
    const message = { jsonrpc: "2.0", id, method };
    if (params !== undefined) message.params = params;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      if (!this.writeMessage(message)) {
        this.pending.delete(id);
        reject(this.failure ?? new ClientFailure(ClientFailureKind.Transport));
      }
    });
  }

  notify(method, params, { queue = true } = {}) {
    if (this.state === "idle" || this.state === "starting") {
      if (queue) this.queuedNotifications.push({ method, params });
      return queue;
    }
    if (!this.child?.stdin.writable || this.state === "stopped" || this.transportClosed) return false;
    return this.writeNotification(method, params);
  }

  writeNotification(method, params) {
    const message = { jsonrpc: "2.0", method };
    if (params !== undefined) message.params = params;
    return this.writeMessage(message);
  }

  flushNotifications() {
    const notifications = this.queuedNotifications.splice(0);
    for (const { method, params } of notifications) this.writeNotification(method, params);
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
      if (message.method === "client/registerCapability" && this.onRegisterCapability) {
        Promise.resolve(this.onRegisterCapability(message.params))
          .then(() => this.respond(message.id, {}))
          .catch((error) => this.respond(message.id, undefined, { code: -32602, message: error.message }));
      } else {
        this.respond(message.id, {});
      }
      return;
    }
    if (message.method === "client/unregisterCapability") {
      Promise.resolve(this.onUnregisterCapability?.(message.params))
        .then(() => this.respond(message.id, {}))
        .catch((error) => this.respond(message.id, undefined, { code: -32602, message: error.message }));
      return;
    }
    if (message.method === "workspace/configuration") {
      this.respond(message.id, []);
      return;
    }
    this.respond(message.id, undefined, { code: -32601, message: `unsupported client method: ${message.method}` });
  }

  respond(id, result, error) {
    if (!this.child?.stdin.writable || this.state === "stopped" || this.transportClosed) return;
    const message = { jsonrpc: "2.0", id };
    if (error) message.error = error;
    else message.result = result;
    this.writeMessage(message);
  }

  writeMessage(message) {
    const stdin = this.child?.stdin;
    if (!stdin?.writable || this.transportClosed) return false;
    try {
      stdin.write(encodeMessage(message), (error) => {
        if (error) this.handleFailure(ClientFailureKind.Transport, error);
      });
      return true;
    } catch (error) {
      this.handleFailure(ClientFailureKind.Transport, error);
      return false;
    }
  }

  async stop() {
    if (!this.child) return;
    if (this.state === "stopped") {
      this.cleanupChild(this.child);
      this.child = undefined;
      this.queuedNotifications = [];
      return;
    }
    if (this.state === "stopping") return;
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
      this.notify("exit", undefined, { queue: false });
      try { child.stdin.end(); } catch { /* already closed */ }
    }
    await Promise.race([exited, timeout(SHUTDOWN_TIMEOUT_MS)]);
    if (!this.exited && !child.killed) {
      try { child.kill(); } catch { /* already gone */ }
      await Promise.race([exited, timeout(SHUTDOWN_TIMEOUT_MS)]);
    }
    this.cleanupChild(child);
    this.rejectPending(new ClientFailure(ClientFailureKind.Lifecycle));
    this.queuedNotifications = [];
    this.child = undefined;
    this.state = "stopped";
  }

  cleanupChild(child) {
    child.stdin.destroy?.();
    child.stdout.destroy?.();
    child.stderr.destroy?.();
  }

  handleFailure(kind, error) {
    const failure = error instanceof ClientFailure
      ? error
      : asClientFailure(kind, error);
    if (this.failure) return false;
    this.failure = failure;
    this.rejectPending(failure);
    this.closeTransport();
    this.emit("failure", failure);
    return true;
  }

  closeTransport() {
    if (this.transportClosed) return;
    this.transportClosed = true;
    const child = this.child;
    child?.stdin.destroy?.();
    if (!this.exited && child && !child.killed) {
      try { child.kill(); } catch { /* already gone */ }
    }
  }

  rejectPending(error) {
    for (const pending of this.pending.values()) pending.reject(error);
    this.pending.clear();
  }
}

function exitDetail(code, signal) {
  const parts = [];
  if (code !== null && code !== undefined) parts.push(`code=${code}`);
  if (signal !== null && signal !== undefined) parts.push(`signal=${signal}`);
  return parts.length ? parts.join(", ") : undefined;
}

function timeout(milliseconds) {
  return new Promise((resolve) => {
    const timer = setTimeout(resolve, milliseconds);
    timer.unref?.();
  });
}
