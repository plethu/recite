import { EventEmitter } from "node:events";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import assert from "node:assert/strict";
import { ReciteLanguageClient } from "../src/lsp-client.js";
import { encodeMessage, LspFrameParser, LspProtocolError } from "../src/lsp-protocol.js";

test("LSP framing survives chunk boundaries and carries multiple messages", () => {
  const messages = [];
  const parser = new LspFrameParser((message) => messages.push(message));
  const payload = Buffer.concat([
    encodeMessage({ jsonrpc: "2.0", id: 1, result: { positionEncoding: "utf-16" } }),
    encodeMessage({ jsonrpc: "2.0", method: "window/logMessage", params: { message: "ok" } })
  ]);
  parser.push(payload.subarray(0, 9));
  parser.push(payload.subarray(9, 27));
  parser.push(payload.subarray(27));
  assert.deepEqual(messages, [
    { jsonrpc: "2.0", id: 1, result: { positionEncoding: "utf-16" } },
    { jsonrpc: "2.0", method: "window/logMessage", params: { message: "ok" } }
  ]);
});

test("LSP framing rejects malformed and oversized lengths", () => {
  const parser = new LspFrameParser(() => {});
  assert.throws(
    () => parser.push(Buffer.from("Content-Length: nope\r\n\r\n{}")),
    LspProtocolError
  );
  assert.throws(
    () => parser.push(Buffer.from("Content-Length: 20000000\r\n\r\n")),
    LspProtocolError
  );
});

test("client performs initialize, answers server lifecycle requests, and shuts down", async () => {
  const child = new FakeProcess();
  const client = new ReciteLanguageClient({
    command: "recite-lsp",
    args: ["--local"],
    cwd: "/workspace",
    spawnProcess: () => child
  });
  const initialized = client.start({
    processId: 12,
    rootUri: "file:///workspace",
    capabilities: { general: { positionEncodings: ["utf-16"] } }
  });
  assert.deepEqual(decode(child.stdin.writes[0]), {
    jsonrpc: "2.0",
    id: 1,
    method: "initialize",
    params: {
      processId: 12,
      rootUri: "file:///workspace",
      capabilities: { general: { positionEncodings: ["utf-16"] } }
    }
  });
  child.stdout.emit("data", encodeMessage({ jsonrpc: "2.0", id: 1, result: { capabilities: {} } }));
  await initialized;
  assert.equal(client.status, "running");
  assert.deepEqual(decode(child.stdin.writes[1]), {
    jsonrpc: "2.0",
    method: "initialized",
    params: {}
  });

  child.stdout.emit("data", encodeMessage({
    jsonrpc: "2.0", id: 42, method: "client/registerCapability", params: { registrations: [] }
  }));
  assert.deepEqual(decode(child.stdin.writes[2]), { jsonrpc: "2.0", id: 42, result: {} });

  client.notify("textDocument/didOpen", { textDocument: { version: 1 } });
  assert.deepEqual(decode(child.stdin.writes[3]), {
    jsonrpc: "2.0", method: "textDocument/didOpen", params: { textDocument: { version: 1 } }
  });

  const stopping = client.stop();
  const shutdown = decode(child.stdin.writes[4]);
  assert.equal(shutdown.method, "shutdown");
  child.stdout.emit("data", encodeMessage({ jsonrpc: "2.0", id: shutdown.id, result: null }));
  await stopping;
  assert.equal(client.status, "stopped");
  assert.deepEqual(decode(child.stdin.writes[5]), { jsonrpc: "2.0", method: "exit" });
  assert.equal(child.killed, false);
});

test("notifications queued during initialize replay after initialized", async () => {
  const child = new FakeProcess();
  const client = new ReciteLanguageClient({ command: "recite-lsp", args: [], spawnProcess: () => child });
  assert.equal(client.notify("textDocument/didOpen", { textDocument: { version: 1 } }), true);
  const starting = client.start({ capabilities: {} });
  assert.deepEqual(decode(child.stdin.writes[0]), {
    jsonrpc: "2.0", id: 1, method: "initialize", params: { capabilities: {} }
  });
  child.stdout.emit("data", encodeMessage({ jsonrpc: "2.0", id: 1, result: {} }));
  await starting;
  assert.equal(decode(child.stdin.writes[1]).method, "initialized");
  assert.equal(decode(child.stdin.writes[2]).method, "textDocument/didOpen");
  const stopping = client.stop();
  const shutdown = decode(child.stdin.writes[3]);
  child.stdout.emit("data", encodeMessage({ jsonrpc: "2.0", id: shutdown.id, result: null }));
  await stopping;
});

test("server capability registration is completed before its response", async () => {
  const child = new FakeProcess();
  const registrations = [];
  const client = new ReciteLanguageClient({
    command: "recite-lsp",
    args: [],
    spawnProcess: () => child,
    onRegisterCapability: async (params) => registrations.push(params)
  });
  const starting = client.start({ capabilities: {} });
  child.stdout.emit("data", encodeMessage({ jsonrpc: "2.0", id: 1, result: {} }));
  await starting;
  child.stdout.emit("data", encodeMessage({
    jsonrpc: "2.0", id: 42, method: "client/registerCapability",
    params: { registrations: [{ id: "watch", method: "workspace/didChangeWatchedFiles" }] }
  }));
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(registrations.length, 1);
  assert.deepEqual(decode(child.stdin.writes[2]), { jsonrpc: "2.0", id: 42, result: {} });
  const stopping = client.stop();
  const shutdown = decode(child.stdin.writes[3]);
  child.stdout.emit("data", encodeMessage({ jsonrpc: "2.0", id: shutdown.id, result: null }));
  await stopping;
});

test("stopping after an unexpected exit cleans up the child transport", async () => {
  const child = new FakeProcess();
  const client = new ReciteLanguageClient({ command: "recite-lsp", args: [], spawnProcess: () => child });
  const starting = client.start({ capabilities: {} });
  child.stdout.emit("data", encodeMessage({ jsonrpc: "2.0", id: 1, result: {} }));
  await starting;
  child.emit("exit", 1, null);
  await new Promise((resolve) => setImmediate(resolve));
  await client.stop();
  assert.equal(child.stdin.destroyed, true);
  assert.equal(child.stdout.destroyed, true);
  assert.equal(child.stderr.destroyed, true);
});

test("an initialize write error is handled exactly once before the child can exit", async () => {
  const child = new FakeProcess({ writeError: Object.assign(new Error("broken pipe"), { code: "EPIPE" }) });
  const failures = [];
  const client = new ReciteLanguageClient({
    command: "recite-lsp", args: [], spawnProcess: () => child
  });
  client.on("failure", (failure) => failures.push(failure));

  await assert.rejects(client.start({ capabilities: {} }), (error) => {
    assert.equal(error.kind, "transport");
    assert.equal(error.code, "EPIPE");
    return true;
  });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(failures.length, 1);
  assert.equal(child.killCount, 1);
  assert.equal(client.pending.size, 0);
});

test("an immediately exiting child rejects initialize with a typed lifecycle failure", async () => {
  const child = new FakeProcess({ immediateExit: true });
  const client = new ReciteLanguageClient({
    command: "recite-lsp", args: [], spawnProcess: () => child
  });

  await assert.rejects(client.start({ capabilities: {} }), (error) => {
    assert.equal(error.kind, "lifecycle");
    assert.equal(error.detail, "code=1");
    return true;
  });
  assert.equal(client.pending.size, 0);
  assert.equal(client.status, "stopped");
});

test("a spawn error without an exit event settles initialize immediately", async () => {
  const child = new FakeProcess({ noExitOnKill: true });
  const client = new ReciteLanguageClient({
    command: "missing-recite-lsp", args: [], spawnProcess: () => child
  });
  const starting = client.start({ capabilities: {} });
  child.emit("error", Object.assign(new Error("ENOENT"), { code: "ENOENT" }));

  await assert.rejects(starting, (error) => {
    assert.equal(error.kind, "lifecycle");
    assert.equal(error.code, "ENOENT");
    return true;
  });
  assert.equal(client.status, "stopped");
  assert.equal(child.killCount, 1);
  assert.equal(child.listenerCount("error"), 0);
  assert.equal(child.listenerCount("exit"), 0);
});

test("a real missing executable spawn error settles without waiting for exit", async () => {
  const command = path.join(os.tmpdir(), `recite-missing-server-${process.pid}`);
  const client = new ReciteLanguageClient({ command, args: [] });
  const startedAt = Date.now();

  await assert.rejects(client.start({ capabilities: {} }), (error) => {
    assert.equal(error.kind, "lifecycle");
    assert.equal(error.code, "ENOENT");
    return true;
  });

  assert.ok(Date.now() - startedAt < 500, "spawn failure should not enter teardown timeouts");
  assert.equal(client.status, "stopped");
});

test("a child error after initialize is terminal and reports one failure without exit", async () => {
  const child = new FakeProcess({ noExitOnKill: true });
  const failures = [];
  const exits = [];
  const client = new ReciteLanguageClient({
    command: "recite-lsp", args: [], spawnProcess: () => child
  });
  client.on("failure", (failure) => failures.push(failure));
  client.on("exit", (event) => exits.push(event));
  const starting = client.start({ capabilities: {} });
  child.stdout.emit("data", encodeMessage({ jsonrpc: "2.0", id: 1, result: {} }));
  await starting;

  child.emit("error", new Error("child failed"));

  assert.equal(client.status, "stopped");
  assert.equal(client.transportClosed, true);
  assert.equal(failures.length, 1);
  assert.equal(failures[0].kind, "lifecycle");
  assert.deepEqual(exits, []);
  await client.stop();
  assert.equal(child.listenerCount("error"), 0);
  assert.equal(child.listenerCount("exit"), 0);
});

for (const [label, emitEvents] of [
  ["error then exit", (child) => {
    child.emit("error", new Error("child failed"));
    child.emit("exit", 1, null);
  }],
  ["exit then error", (child) => {
    child.emit("exit", 1, null);
    child.emit("error", new Error("late child failure"));
  }]
]) {
  test(`child ${label} produces one failure and one exit notification`, async () => {
    const child = new FakeProcess({ noExitOnKill: true });
    const failures = [];
    const exits = [];
    const client = new ReciteLanguageClient({
      command: "recite-lsp", args: [], spawnProcess: () => child
    });
    client.on("failure", (failure) => failures.push(failure));
    client.on("exit", (event) => exits.push(event));
    const starting = client.start({ capabilities: {} });
    child.stdout.emit("data", encodeMessage({ jsonrpc: "2.0", id: 1, result: {} }));
    await starting;

    emitEvents(child);

    assert.equal(client.status, "stopped");
    assert.equal(failures.length, 1);
    assert.deepEqual(exits, [{ code: 1, signal: null }]);
    await client.stop();
  });
}

test("graceful stop forces at one total deadline and concurrent callers join it", async () => {
  const clock = new FakeClock();
  const child = new FakeProcess({ noExitOnKill: true });
  const client = new ReciteLanguageClient({
    command: "recite-lsp", args: [], spawnProcess: () => child, clock
  });
  const starting = client.start({ capabilities: {} });
  child.stdout.emit("data", encodeMessage({ jsonrpc: "2.0", id: 1, result: {} }));
  await starting;

  const first = client.stop();
  const second = client.stop();
  assert.strictEqual(first, second);
  await flushMicrotasks();
  assert.equal(client.status, "stopping");
  assert.equal(decode(child.stdin.writes.at(-1)).method, "shutdown");
  assert.equal(clock.timers.length, 1);
  assert.equal(child.killCount, 0);
  clock.advance(999);
  await flushMicrotasks();
  assert.equal(child.killCount, 0);
  clock.advance(1);
  await first;

  assert.equal(child.killCount, 1);
  assert.equal(client.status, "stopped");
  assert.equal(child.listenerCount("error"), 0);
  assert.equal(child.listenerCount("exit"), 0);
  assert.equal(child.stdin.listenerCount("error"), 0);
});

test("a synchronous failure observer cannot orphan the captured child", async () => {
  const child = new FakeProcess({ noExitOnKill: true });
  const client = new ReciteLanguageClient({
    command: "recite-lsp", args: [], spawnProcess: () => child
  });
  let observerStop;
  client.on("failure", () => { observerStop = client.stop(); });
  const starting = client.start({ capabilities: {} });
  child.stdout.emit("data", encodeMessage({ jsonrpc: "2.0", id: 1, result: {} }));
  await starting;

  child.emit("error", new Error("child failed"));
  await observerStop;

  assert.equal(child.killCount, 1);
  assert.equal(client.status, "stopped");
  assert.equal(child.stdin.destroyed, true);
});

test("child errors during teardown are ignored and remain within the deadline", async () => {
  const clock = new FakeClock();
  const child = new FakeProcess({ noExitOnKill: true });
  const failures = [];
  const client = new ReciteLanguageClient({
    command: "recite-lsp", args: [], spawnProcess: () => child, clock
  });
  client.on("failure", (failure) => failures.push(failure));
  const starting = client.start({ capabilities: {} });
  child.stdout.emit("data", encodeMessage({ jsonrpc: "2.0", id: 1, result: {} }));
  await starting;

  const stopping = client.stop();
  await flushMicrotasks();
  child.emit("error", new Error("late child failure during stop"));
  clock.advance(1000);
  await stopping;

  assert.deepEqual(failures, []);
  assert.equal(child.killCount, 1);
  assert.equal(client.status, "stopped");
});

test("a transport error wins a response race and rejects every pending request once", async () => {
  const child = new FakeProcess();
  const failures = [];
  const client = new ReciteLanguageClient({
    command: "recite-lsp", args: [], spawnProcess: () => child
  });
  client.on("failure", (failure) => failures.push(failure));
  const starting = client.start({ capabilities: {} });
  child.stdout.emit("data", encodeMessage({ jsonrpc: "2.0", id: 1, result: {} }));
  await starting;

  const request = client.request("workspace/symbol", {});
  const requestId = decode(child.stdin.writes.at(-1)).id;
  child.stdin.emit("error", Object.assign(new Error("broken pipe"), { code: "EPIPE" }));
  child.stdout.emit("data", encodeMessage({ jsonrpc: "2.0", id: requestId, result: [] }));
  await assert.rejects(request, (error) => error.kind === "transport");
  assert.equal(failures.length, 1);
  assert.equal(client.pending.size, 0);
  await client.stop();
});

class FakeStream extends EventEmitter {
  constructor(onEnd, options = {}) {
    super();
    this.onEnd = onEnd;
    this.writable = true;
    this.destroyed = false;
    this.writes = [];
    this.writeError = options.writeError;
    this.onWrite = options.onWrite;
  }

  write(value, callback) {
    if (this.writeError) {
      const error = this.writeError;
      this.writeError = undefined;
      queueMicrotask(() => callback?.(error));
      return true;
    }
    this.writes.push(Buffer.from(value));
    this.onWrite?.(Buffer.from(value));
    callback?.();
    return true;
  }

  end() {
    this.writable = false;
    this.onEnd?.();
  }

  destroy() {
    this.writable = false;
    this.destroyed = true;
  }
}

class FakeProcess extends EventEmitter {
  constructor(options = {}) {
    super();
    this.noExitOnKill = options.noExitOnKill ?? false;
    this.stdin = new FakeStream(
      () => {
        if (!options.noExitOnEnd) queueMicrotask(() => this.emit("exit", 0, null));
      }, options
    );
    this.stdout = new FakeStream();
    this.stderr = new FakeStream();
    this.killed = false;
    this.killCount = 0;
    if (options.immediateExit) queueMicrotask(() => this.emit("exit", 1, null));
  }

  kill() {
    this.killCount += 1;
    this.killed = true;
    if (!this.noExitOnKill) queueMicrotask(() => this.emit("exit", 0, null));
  }
}

class FakeClock {
  constructor() {
    this.time = 0;
    this.timers = [];
  }

  now = () => this.time;

  setTimeout = (callback, milliseconds) => {
    const timer = { callback, due: this.time + milliseconds, cancelled: false };
    this.timers.push(timer);
    return timer;
  };

  clearTimeout = (timer) => { timer.cancelled = true; };

  advance(milliseconds) {
    this.time += milliseconds;
    const due = [];
    const remaining = [];
    for (const timer of this.timers) {
      if (timer.cancelled) continue;
      if (timer.due <= this.time) due.push(timer);
      else remaining.push(timer);
    }
    this.timers = remaining;
    for (const timer of due) timer.callback();
  }
}

async function flushMicrotasks() {
  await new Promise((resolve) => setImmediate(resolve));
}

function decode(frame) {
  const separator = frame.indexOf(Buffer.from("\r\n\r\n"));
  return JSON.parse(frame.subarray(separator + 4).toString("utf8"));
}
