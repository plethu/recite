import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { mkdtemp, mkdir, rm, symlink } from "node:fs/promises";
import os from "node:os";
import { assertSafeTree } from "../scripts/safety.mjs";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const manifest = JSON.parse(await readFile(path.join(packageRoot, "package.json"), "utf8"));

test("the shared artifact serves both VS Code and VSCodium without semantic forks", () => {
  assert.equal(manifest.name, "recite-vscode");
  assert.equal(manifest.main, "./dist/extension.js");
  assert.deepEqual(manifest.contributes.languages[0].extensions, [".recite"]);
  assert.equal(manifest.contributes.grammars, undefined);
  assert.equal(manifest.contributes.commands, undefined);
  assert.match(manifest.repository.url, /github\.com\/plethu\/recite\.git$/);
});

test("packaging safety rejects symlink escapes, including intermediate paths", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "recite-vscode-safety-"));
  try {
    await mkdir(path.join(root, "nested"));
    await symlink("/tmp", path.join(root, "nested", "escape"));
    assert.throws(() => assertSafeTree(root), /symlink/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
