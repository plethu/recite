import { EventEmitter } from "node:events";
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
  }

  write(value, callback) {
    if (this.writeError) {
      const error = this.writeError;
      this.writeError = undefined;
      queueMicrotask(() => callback?.(error));
      return true;
    }
    this.writes.push(Buffer.from(value));
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
    this.stdin = new FakeStream(
      () => queueMicrotask(() => this.emit("exit", 0, null)), options
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
    queueMicrotask(() => this.emit("exit", 0, null));
  }
}

function decode(frame) {
  const separator = frame.indexOf(Buffer.from("\r\n\r\n"));
  return JSON.parse(frame.subarray(separator + 4).toString("utf8"));
}
