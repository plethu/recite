import assert from "node:assert/strict";
import test, { type TestContext } from "node:test";
import { envelope, FakeChild, started, watchRegistry } from "./watch-test-fixtures.mjs";

test("dispose recovers a child that never emits a terminal record", async (context) => {
  context.mock.timers.enable({ apis: ["setTimeout"] });
  const messages: unknown[][] = [];
  const child = new FakeChild();
  const registry = watchRegistry(messages, child, "dispose-id", 10);
  registry.register([]);
  await registry.api.commands.executeCommand("recite.watch.start");
  const disposed = registry.dispose();
  await advance(context, 25);
  await disposed;
  assert.equal(child.killed, true);
  assert.equal(registry.watch.active, undefined);
});

test("watch recovery keeps ownership until force-kill close", async (context) => {
  context.mock.timers.enable({ apis: ["setTimeout"] });
  const messages: unknown[][] = [];
  const child = new FakeChild();
  const registry = watchRegistry(messages, child, "tombstone-id", 5);
  registry.register([]);
  await registry.api.commands.executeCommand("recite.watch.start");
  const stopping = registry.api.commands.executeCommand("recite.watch.stop");
  await advance(context, 8);
  assert.equal(registry.watch.active !== undefined, true);
  assert.equal(await registry.api.commands.executeCommand("recite.watch.start"), undefined);
  assert.equal(messages.at(-1)?.[0], "running");
  await advance(context, 10);
  await stopping;
  assert.equal(registry.watch.active, undefined);
});

test("fatal terminal enters bounded recovery without releasing a non-closing child", async (context) => {
  context.mock.timers.enable({ apis: ["setTimeout"] });
  const messages: unknown[][] = [];
  const child = new UncooperativeChild();
  const registry = watchRegistry(messages, child, "fatal-hung-id", 5);
  registry.register([]);
  await registry.api.commands.executeCommand("recite.watch.start");
  child.stdout.emit(
    "data",
    Buffer.from(
      JSON.stringify(started("fatal-hung-id", 0)) + "\n"
        + JSON.stringify(envelope("fatal-hung-id", 1, "watch.stopped", {
          reason: { type: "fatal" },
          error: { category: "input", code: "missing_path", operation: "watch" },
        })) + "\n",
    ),
  );
  await advance(context, 30);
  assert.deepEqual(child.signals, ["SIGTERM", "SIGKILL"]);
  assert.notEqual(registry.watch.active, undefined);
  assert.equal(await registry.api.commands.executeCommand("recite.watch.start"), undefined);
  assert.equal(messages.at(-1)?.[0], "running");
  child.close(1);
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(registry.watch.active, undefined);
});

class UncooperativeChild extends FakeChild {
  signals: NodeJS.Signals[] = [];

  override kill(signal: NodeJS.Signals = "SIGTERM") {
    this.killed = true;
    this.signals.push(signal);
    return true;
  }
}

// Flush promise continuations between ticks so force timers retain their order.
async function advance(context: TestContext, milliseconds: number) {
  for (let elapsed = 0; elapsed < milliseconds; elapsed++) {
    context.mock.timers.tick(1);
    await new Promise<void>((resolve) => setImmediate(resolve));
  }
}
