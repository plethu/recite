import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const manifest = JSON.parse(await readFile(path.join(packageRoot, "package.json"), "utf8"));
const main = path.resolve(packageRoot, manifest.main);
const require = createRequire(import.meta.url);

test("the declared main loads as CommonJS and delegates to the ESM implementation", async () => {
  const entry = require(main);
  assert.equal(typeof entry.activate, "function");
  assert.equal(typeof entry.deactivate, "function");

  const beforeActivation = entry.deactivate();
  assert(beforeActivation instanceof Promise, "deactivate must preserve the VS Code promise contract");
  await beforeActivation;

  const root = await mkdtemp(path.join(os.tmpdir(), "recite-vscode-entry-"));
  try {
    await writeFile(path.join(root, "package.json"), '{"type":"module"}\n');
    await writeFile(path.join(root, "extension.cjs"), await readFile(main));
    await writeFile(path.join(root, "extension.js"), `
      export const calls = [];
      export function activate(context) {
        calls.push(["activate", context]);
        return "activated";
      }
      export function deactivate(reason) {
        calls.push(["deactivate", reason]);
        return "deactivated";
      }
    `);

    const temporaryEntry = createRequire(path.join(root, "test.cjs"))(path.join(root, "extension.cjs"));
    const context = { subscriptions: [] };
    assert.equal(await temporaryEntry.activate(context), "activated");
    assert.equal(await temporaryEntry.deactivate("test cleanup"), "deactivated");

    const implementation = await import(pathToFileURL(path.join(root, "extension.js")).href);
    assert.deepEqual(implementation.calls, [
      ["activate", context],
      ["deactivate", "test cleanup"]
    ]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
