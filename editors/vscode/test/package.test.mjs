import test from "node:test";
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { mkdtemp, mkdir, rm, symlink } from "node:fs/promises";
import os from "node:os";
import { assertContainedRegularFile, assertSafeTree } from "../scripts/safety.mjs";

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

test("packaging safety rejects a symlinked or escaping repository license", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "recite-vscode-license-"));
  try {
    await writeFile(path.join(root, "outside"), "license");
    await symlink(path.join(root, "outside"), path.join(root, "LICENSE"));
    assert.throws(() => assertContainedRegularFile(root, "LICENSE", "repository license"), /symlink/);
    assert.throws(() => assertContainedRegularFile(root, "../outside", "repository license"), /outside/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
