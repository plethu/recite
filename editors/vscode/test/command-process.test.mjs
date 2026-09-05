import test from "node:test";
import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { runFiniteCommand } from "../src/command-process.js";

test("finite adapters accept content diagnostics and typed failures", async () => {
  const content = await runFinite("command.result", 1, {
    status: "content_diagnostics", exit_code: 1, data: { diagnostics: [] }
  });
  assert.equal(content.terminal.status, "content_diagnostics");

  const failure = await runFinite("command.error", 1, {
    status: "failure", exit_code: 1,
    error: { category: "input", code: "read", operation: "validate" }
  });
  assert.equal(failure.terminal.error.category, "input");
});

test("finite adapters accept a producer-valid asset-load failure", async () => {
  const failure = await runFinite("command.error", 1, {
    status: "failure", exit_code: 1,
    error: { category: "asset", code: "decode_asset", operation: "load_asset" }
  }, "run");
  assert.equal(failure.terminal.error.operation, "load_asset");
});

test("structured stderr is a protocol failure and is never parsed as display text", async () => {
  const child = new FakeChild();
  const promise = runFiniteWithChild(child);
  child.stderr.emit("data", Buffer.from("human diagnostic\n"));
  await assert.rejects(promise, /structured_stderr/);
  assert.equal(child.killed, true);
});

test("asynchronous stdin failures are contained as process failures", async () => {
  const child = new FakeChild();
  const promise = runFiniteWithChild(child);
  child.stdin.emit("error", new Error("EPIPE"));
  await assert.rejects(promise, /EPIPE/);
});

async function runFinite(event, exitCode, terminal, command = "validate") {
  const child = new FakeChild();
  const promise = runFiniteWithChild(child, command);
  queueMicrotask(() => {
    child.stdout.emit("data", Buffer.from(JSON.stringify({
      version: 1, sequence: 0, event: "command.started", command, invocation_id: "id"
    }) + "\n" + JSON.stringify({
      version: 1, sequence: 1, event, command, invocation_id: "id", ...terminal
    }) + "\n"));
    child.close(exitCode);
  });
  return promise;
}

function runFiniteWithChild(child, command = "validate") {
  return runFiniteCommand({
    command: "recite", commandName: command, args: [], cwd: "/project", invocationId: "id",
    spawnProcess: () => child
  });
}

class FakeChild extends EventEmitter {
  constructor() {
    super();
    this.stdout = new EventEmitter();
    this.stderr = new EventEmitter();
    this.stdin = new EventEmitter();
    this.stdin.end = () => {};
    this.stdin.destroy = () => {};
    this.killed = false;
  }

  kill() { this.killed = true; }
  close(code) { this.emit("close", code, null); }
}
