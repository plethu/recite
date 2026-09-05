import test from "node:test";
import assert from "node:assert/strict";
import { FakeChild, started, watchDiagnostic, watchRegistry, cancel, stopped,
  completed, envelope } from "./watch-test-fixtures.mjs";

test("idle watch stop sends cancellation and waits for stopped plus exit", async () => {
  const messages = [];
  const child = new FakeChild();
  const registry = watchRegistry(messages, child, "idle-id");
  registry.register([]);
  await registry.api.commands.executeCommand("recite.watch.start");
  child.stdout.emit("data", Buffer.from(JSON.stringify(started("idle-id", 0)) + "\n"));
  const stopping = registry.api.commands.executeCommand("recite.watch.stop");
  assert.deepEqual(child.stdin.writes[0], {
    version: 1, command: "watch", action: "cancel", invocation_id: "idle-id"
  });
  child.stdout.emit("data", Buffer.from(JSON.stringify(cancel("idle-id", 1)) + "\n" +
    JSON.stringify(stopped("idle-id", 2)) + "\n"));
  child.close(0);
  assert.deepEqual(await stopping, { stopped: true, exitCode: 0 });
});

test("active watch stop accepts cancellation before the cancelled build completes", async () => {
  const messages = [];
  const child = new FakeChild();
  const registry = watchRegistry(messages, child, "active-id");
  registry.register([]);
  await registry.api.commands.executeCommand("recite.watch.start");
  child.stdout.emit("data", Buffer.from(JSON.stringify(started("active-id", 0)) + "\n" +
    JSON.stringify(envelope("active-id", 1, "watch.build.started", {
      generation: 0, trigger: "initial"
    })) + "\n"));
  const stopping = registry.api.commands.executeCommand("recite.watch.stop");
  child.stdout.emit("data", Buffer.from(JSON.stringify(cancel("active-id", 2)) + "\n" +
    JSON.stringify(completed("active-id", 3)) + "\n" +
    JSON.stringify(stopped("active-id", 4)) + "\n"));
  child.close(0);
  assert.deepEqual(await stopping, { stopped: true, exitCode: 0 });
});

test("recoverable control errors are visible and do not retire the watch", async () => {
  const messages = [];
  const child = new FakeChild();
  const registry = watchRegistry(messages, child, "control-id");
  registry.register([]);
  await registry.api.commands.executeCommand("recite.watch.start");
  child.stdout.emit("data", Buffer.from(JSON.stringify(started("control-id", 0)) + "\n" +
    JSON.stringify(envelope("control-id", 1, "watch.control.error", {
      error: { type: "malformed" }
    })) + "\n"));
  assert.equal(registry.watch.active !== undefined, true);
  assert.equal(messages.at(-1)[0], "watch");
  const stopping = registry.api.commands.executeCommand("recite.watch.stop");
  child.stdout.emit("data", Buffer.from(JSON.stringify(cancel("control-id", 2)) + "\n" +
    JSON.stringify(stopped("control-id", 3)) + "\n"));
  child.close(0);
  await stopping;
});

test("fatal watch stops are visible as command failures", async () => {
  const messages = [];
  const child = new FakeChild();
  const registry = watchRegistry(messages, child, "fatal-id");
  registry.register([]);
  await registry.api.commands.executeCommand("recite.watch.start");
  child.stdout.emit("data", Buffer.from(JSON.stringify(started("fatal-id", 0)) + "\n" +
    JSON.stringify(envelope("fatal-id", 1, "watch.stopped", {
      reason: { type: "fatal" },
      error: { category: "input", code: "missing_path", operation: "watch" }
    })) + "\n"));
  child.close(1);
  assert.equal(messages.at(-1)[0], "failure");
});

test("a crashed watch retires ownership and ignores late records", async () => {
  const messages = [];
  const child = new FakeChild();
  const registry = watchRegistry(messages, child, "crash-id");
  registry.register([]);
  await registry.api.commands.executeCommand("recite.watch.start");
  child.stdout.emit("data", Buffer.from(JSON.stringify(started("crash-id", 0)) + "\n"));
  child.close(1);
  assert.equal(registry.watch.active, undefined);
  const count = messages.length;
  child.stdout.emit("data", Buffer.from(JSON.stringify(started("crash-id", 0)) + "\n"));
  assert.equal(messages.length, count);
  assert.equal(messages.at(-1)[0], "protocol");
});

test("watch diagnostics use the started project root authority", async () => {
  const messages = [];
  const child = new FakeChild();
  const registry = watchRegistry(messages, child, "root-id");
  registry.register([]);
  await registry.api.commands.executeCommand("recite.watch.start");
  child.stdout.emit("data", Buffer.from(JSON.stringify(started("root-id", 0)) + "\n"));
  assert.equal(registry.watch.active.projectRoot, "/project");
  const stopping = registry.api.commands.executeCommand("recite.watch.stop");
  child.stdout.emit("data", Buffer.from(JSON.stringify(cancel("root-id", 1)) + "\n" +
    JSON.stringify(stopped("root-id", 2)) + "\n"));
  child.close(0);
  await stopping;
});

test("watch rejects a started project root that differs from launch authority", async () => {
  const messages = [];
  const child = new FakeChild();
  const registry = watchRegistry(messages, child, "mismatch-id", 1_500, "nested");
  registry.register([]);
  await registry.api.commands.executeCommand("recite.watch.start");
  child.stdout.emit("data", Buffer.from(JSON.stringify(started("mismatch-id", 0)) + "\n"));
  assert.equal(messages.at(-1)[0], "protocol");
  child.close(1);
  await registry.dispose();
});

test("watch diagnostics do not attach a disk snapshot to a dirty overlay", async () => {
  const messages = [];
  const document = {
    isDirty: true,
    uri: { scheme: "file", fsPath: "/project/dialogue.recite", toString: () => "file:///project/dialogue.recite" },
    getText: () => "😀unsaved overlay"
  };
  const child = new FakeChild();
  const registry = watchRegistry(messages, child, "dirty-id", 1_500, "", [document]);
  registry.register([]);
  await registry.api.commands.executeCommand("recite.watch.start");
  child.stdout.emit("data", Buffer.from(JSON.stringify(started("dirty-id", 0)) + "\n"));
  registry.replaceWatchDiagnostics(registry.watch.active, [watchDiagnostic("dialogue.recite")], "/project");
  assert.equal(registry.diagnosticUris.size, 0);
  child.close(1);
  await registry.dispose();
});
