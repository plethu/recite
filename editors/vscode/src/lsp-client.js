import { EventEmitter } from "node:events";
import { spawn } from "node:child_process";
import { LspFrameParser } from "./lsp-protocol.js";
import {
  asClientFailure,
  ClientFailure,
  ClientFailureKind
} from "./client-failure.js";
import { closeTransport, writeFramedMessage } from "./lsp-transport.js";

const SHUTDOWN_TIMEOUT_MS = 1000;

export class ReciteLanguageClient extends EventEmitter {
  constructor(options) {
    super();
    this.command = options.command;
    this.args = [...options.args];
    this.cwd = options.cwd;
    this.environment = options.environment;
    this.spawnProcess = options.spawnProcess ?? spawn;
    const clock = options.clock ?? {};
    this.clock = {
      now: clock.now ?? options.now ?? (() => Date.now()),
      setTimeout: clock.setTimeout ?? options.setTimeout ?? ((callback, milliseconds) =>
        setTimeout(callback, milliseconds)),
      clearTimeout: clock.clearTimeout ?? options.clearTimeout ?? ((timer) => clearTimeout(timer))
    };
    this.onRegisterCapability = options.onRegisterCapability;
    this.onUnregisterCapability = options.onUnregisterCapability;
    this.child = undefined;
    this.state = "idle";
    this.nextRequestId = 1;
    this.pending = new Map();
    this.queuedNotifications = [];
    this.transportClosed = false;
    this.failure = undefined;
    this.exited = false;
    this.childListeners = undefined;
    this.stopPromise = undefined;
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
    this.transportClosed = false;
    this.failure = undefined;
    this.exited = false;
    this.attachChild(this.child);

    try {
      const result = await this.request("initialize", initializeParams);
      if (this.state !== "starting" || this.failure || this.exited || this.transportClosed) {
        throw this.failure ?? new ClientFailure(ClientFailureKind.Lifecycle);
      }
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
    if (this.transportClosed) return false;
    return writeFramedMessage(
      this.child?.stdin, message, (error) => this.handleFailure(ClientFailureKind.Transport, error)
    );
  }

  stop() {
    if (this.stopPromise) return this.stopPromise;
    if (!this.child) return Promise.resolve();
    if (this.state === "stopped") {
      const child = this.child;
      this.cleanupChild(child);
      this.child = undefined;
      this.queuedNotifications = [];
      return Promise.resolve();
    }
    this.state = "stopping";
    const child = this.child;
    this.stopPromise = this.teardown(child).finally(() => {
      this.stopPromise = undefined;
    });
    return this.stopPromise;
  }

  async teardown(child) {
    const exitWait = this.waitForExit(child);
    const exited = exitWait.promise;
    const deadline = createDeadline(this.clock, SHUTDOWN_TIMEOUT_MS);
    const deadlineReached = deadline.promise.then(() => "deadline");
    const exitReached = exited.then(() => "exit");

    try {
      if (!this.exited && child.stdin.writable && !this.transportClosed) {
        let shutdown;
        try {
          shutdown = this.request("shutdown", undefined);
        } catch {
          shutdown = Promise.resolve();
        }
        const shutdownReached = Promise.resolve(shutdown)
          .then(() => "shutdown", () => "shutdown");
        const result = await Promise.race([shutdownReached, exitReached, deadlineReached]);
        if (result !== "deadline" && result !== "exit" && !this.exited) {
          this.notify("exit", undefined, { queue: false });
          try { child.stdin.end(); } catch { /* already closed */ }
        }
      }

      await Promise.race([exitReached, deadlineReached]);
      if (!this.exited && !child.killed) {
        try { child.kill(); } catch { /* already gone */ }
      }
      await Promise.race([exitReached, deadlineReached]);
    } finally {
      exitWait.cancel();
      deadline.cancel();
      this.cleanupChild(child);
      this.rejectPending(new ClientFailure(ClientFailureKind.Lifecycle));
      this.queuedNotifications = [];
      if (this.child === child) this.child = undefined;
      this.state = "stopped";
    }
  }

  waitForExit(child) {
    if (this.exited) return { promise: Promise.resolve(), cancel() {} };
    let settled = false;
    let onExit;
    const promise = new Promise((resolve) => {
      onExit = () => {
        settled = true;
        child.removeListener("exit", onExit);
        resolve();
      };
      child.once("exit", onExit);
    });
    return {
      promise,
      cancel: () => {
        if (!settled) child.removeListener("exit", onExit);
      }
    };
  }

  attachChild(child) {
    const listeners = {
      stdout: (chunk) => this.read(chunk),
      stderr: (chunk) => this.emit("stderr", chunk.toString("utf8")),
      stdinError: (error) => this.handleFailure(ClientFailureKind.Transport, error),
      error: (error) => this.handleFailure(ClientFailureKind.Lifecycle, error),
      exit: (code, signal) => this.handleExit(child, code, signal)
    };
    child.stdout.on("data", listeners.stdout);
    child.stderr.on("data", listeners.stderr);
    // A Writable emits EPIPE asynchronously. Attach this before the first
    // initialize write so a child that exits immediately cannot surface an
    // uncaught exception or leave a pending request hanging.
    child.stdin.on("error", listeners.stdinError);
    child.on("error", listeners.error);
    child.on("exit", listeners.exit);
    this.childListeners = { child, listeners };
  }

  handleExit(child, code, signal) {
    if (this.child !== child || this.exited) return;
    this.exited = true;
    const unexpected = this.state !== "stopping" && this.state !== "stopped";
    this.state = "stopped";
    this.closeTransport();
    if (unexpected && !this.failure) {
      this.failure = new ClientFailure(ClientFailureKind.Lifecycle, exitDetail(code, signal));
      this.rejectPending(this.failure);
      this.emit("failure", this.failure);
    }
    this.emit("exit", { code, signal });
  }

  cleanupChild(child) {
    if (this.childListeners?.child === child) {
      const { listeners } = this.childListeners;
      child.stdout.removeListener("data", listeners.stdout);
      child.stderr.removeListener("data", listeners.stderr);
      child.stdin.removeListener("error", listeners.stdinError);
      child.removeListener("error", listeners.error);
      child.removeListener("exit", listeners.exit);
      this.childListeners = undefined;
    }
    child.stdin.destroy?.();
    child.stdout.destroy?.();
    child.stderr.destroy?.();
  }

  handleFailure(kind, error) {
    const failure = error instanceof ClientFailure
      ? error
      : asClientFailure(kind, error);
    if (this.failure || this.exited || this.state === "stopping" || this.state === "stopped") {
      return false;
    }
    this.failure = failure;
    this.rejectPending(failure);
    this.state = "stopped";
    const child = this.child;
    this.emit("failure", failure);
    // Failure observers may synchronously stop the client and clear
    // `this.child`; close the captured process rather than rereading mutable
    // client state after the event.
    this.closeTransport(child);
    return true;
  }

  closeTransport(child = this.child) {
    if (this.transportClosed) return;
    this.transportClosed = true;
    closeTransport(child, child?.stdin, this.exited);
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

function createDeadline(clock, milliseconds) {
  let timer;
  let settled = false;
  const promise = new Promise((resolve) => {
    const remaining = Math.max(0, milliseconds);
    timer = clock.setTimeout(() => {
      settled = true;
      resolve();
    }, remaining);
    timer?.unref?.();
  });
  return {
    promise,
    cancel: () => {
      if (settled) return;
      settled = true;
      if (timer !== undefined) clock.clearTimeout(timer);
    }
  };
}
