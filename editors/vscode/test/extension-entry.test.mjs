import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { test } from "bun:test";

test("the CommonJS entry injects the host API into the built ESM lifecycle in Node", () => {
  const probe = spawnSync(
    "node",
    [fileURLToPath(new URL("./extension-entry-probe.mjs", import.meta.url))],
    { encoding: "utf8" },
  );
  assert.equal(probe.status, 0, `${probe.stdout}\n${probe.stderr}`);
});
