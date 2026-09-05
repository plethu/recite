import test from "node:test";
import assert from "node:assert/strict";
import { FakeChild, envelope, started, watchRegistry } from "./watch-test-fixtures.mjs";

test("dispose recovers a child that never emits a terminal record", async () => {
  const messages = [];
  const child = new FakeChild();
  const registry = watchRegistry(messages, child, "dispose-id", 10);
  registry.register([]);
  await registry.api.commands.executeCommand("recite.watch.start");
  const disposed = registry.dispose();
  await new Promise((resolve) => setTimeout(resolve, 25));
  await disposed;
  assert.equal(child.killed, true);
  assert.equal(registry.watch.active, undefined);
});

test("watch recovery keeps ownership until force-kill close", async () => {
  const messages = [];
  const child = new FakeChild();
  const registry = watchRegistry(messages, child, "tombstone-id", 5);
  registry.register([]);
  await registry.api.commands.executeCommand("recite.watch.start");
  const stopping = registry.api.commands.executeCommand("recite.watch.stop");
  await new Promise((resolve) => setTimeout(resolve, 8));
  assert.equal(registry.watch.active !== undefined, true);
  assert.equal(await registry.api.commands.executeCommand("recite.watch.start"), undefined);
  assert.equal(messages.at(-1)[0], "running");
  await stopping;
  assert.equal(registry.watch.active, undefined);
});

test("fatal terminal enters bounded recovery without releasing a non-closing child", async () => {
  const messages = [];
  const child = new UncooperativeChild();
  const registry = watchRegistry(messages, child, "fatal-hung-id", 5);
  registry.register([]);
  await registry.api.commands.executeCommand("recite.watch.start");
  child.stdout.emit("data", Buffer.from(
    JSON.stringify(started("fatal-hung-id", 0)) + "\n" +
    JSON.stringify(envelope("fatal-hung-id", 1, "watch.stopped", {
      reason: { type: "fatal" },
      error: { category: "input", code: "missing_path", operation: "watch" }
    })) + "\n"
  ));
  await new Promise((resolve) => setTimeout(resolve, 30));
  assert.deepEqual(child.signals, ["SIGTERM", "SIGKILL"]);
  assert.notEqual(registry.watch.active, undefined);
  assert.equal(await registry.api.commands.executeCommand("recite.watch.start"), undefined);
  assert.equal(messages.at(-1)[0], "running");
  child.close(1);
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(registry.watch.active, undefined);
});

class UncooperativeChild extends FakeChild {
  constructor() {
    super();
    this.signals = [];
  }

  kill(signal) {
    this.killed = true;
    this.signals.push(signal);
  }
}
