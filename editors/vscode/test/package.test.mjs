import test from "node:test";
import assert from "node:assert/strict";
import { readFile, writeFile, readdir, mkdtemp, mkdir, rm, symlink } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import os from "node:os";
import { assertContainedRegularFile, assertSafeTree } from "../scripts/safety.mjs";
import { assertUiBoundary } from "../scripts/ui-boundary.mjs";
import { SOURCE_MESSAGE_IDS } from "../scripts/message-projections.mjs";
import projectedMessages from "../src/messages.generated.js";

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

test("the declared VS Code floor uses a plain JavaScript message projection", async () => {
  assert.equal(manifest.engines.vscode, "^1.89.0");
  const source = await readFile(path.join(packageRoot, "src", "messages.js"), "utf8");
  assert.match(source, /messages\.generated\.js/);
  assert.doesNotMatch(source, /\.json.*import attributes|with \{ type: ["']json["'] \}/);
  const generated = await readFile(path.join(packageRoot, "src", "messages.generated.js"), "utf8");
  assert.match(generated, /^\/\/ Generated from .*\.ftl/m);
});

test("the UI adapter owns source messages and syntax decoys remain inert", async () => {
  const entries = await sourceEntries();
  assert.doesNotThrow(() => assertUiBoundary(entries, SOURCE_MESSAGE_IDS, projectedMessages));
  const decoys = [
    String.raw`const regex = /clientMessage(api, "lsp-client-start-failed")/;`,
    String.raw`const text = "clientMessage(api, lsp-client-start-failed)";`,
    String.raw`// clientMessage(api, "lsp-client-start-failed")`,
    "const template = `clientMessage(api, \\\"lsp-client-start-failed\\\")`;"
  ].join("\n");
  assert.doesNotThrow(() => assertUiBoundary(
    [...entries, ["decoys.js", decoys]], SOURCE_MESSAGE_IDS, projectedMessages
  ));
});

test("the outside policy rejects wrapper, sink, alias, and dynamic bypasses", async () => {
  const entries = await sourceEntries();
  const rejected = (source) => assert.throws(() => assertUiBoundary(
    [...entries, ["hostile.js", source]], SOURCE_MESSAGE_IDS, projectedMessages
  ), /outside|acquisition|access|import|call|projection/);
  for (const source of [
    String.raw`function fake(clientMessage) { clientMessage(api, "lsp-client-start-failed"); }`,
    String.raw`const clientMessage = () => {}; clientMessage(api, "lsp-client-start-failed");`,
    String.raw`let emit; emit = clientMessage;`,
    String.raw`const output = {}; output.appendLine("English");`,
    String.raw`const host = {}; host.append("English");`,
    String.raw`const output = {}; output["appendLine"]("English");`,
    String.raw`const output = {}; const method = "appendLine"; output[method]("English");`,
    String.raw`const output = {}; const emit = output.appendLine; emit("English");`,
    String.raw`const output = {}; const { appendLine: emit } = output; emit("English");`,
    String.raw`const output = {}; const emit = output.appendLine.call(output); emit("English");`,
    String.raw`const output = {}; const emit = output.appendLine.apply(output, ["English"]);`,
    String.raw`const output = {}; const emit = output.appendLine.bind(output); emit("English");`,
    String.raw`const output = {}; output.appendLine(...["English"]);`,
    String.raw`const output = {}; output.appendLine(condition ? "a" : "b");`,
    String.raw`const output = {}; output.appendLine("prefix " + detail);`,
    String.raw`const api = { window: {} }; api.window.showWarningMessage("Warning");`,
    String.raw`const api = { window: {} }; api.window.showErrorMessage("Error");`,
    String.raw`const api = { window: {} }; api.window.showInformationMessage("Information");`,
    String.raw`const api = { window: {} }; api.window[method]("Warning");`,
    String.raw`const ui = {}; ui[method]("Warning");`,
    String.raw`const controller = { userInterface: {} }; controller.userInterface[method]("Warning");`,
    String.raw`import { clientMessage } from "./messages.js";`,
    String.raw`const api = {}; api.window.createOutputChannel("name");`
  ]) rejected(source);
  assert.doesNotThrow(() => assertUiBoundary([
    ...entries, ["builder.js", "const builder = { append() {} }; builder.append(value);"]
  ], SOURCE_MESSAGE_IDS, projectedMessages));
});

test("the adapter rejects escaped IDs, aliases, reassignment, and composed text", async () => {
  const entries = await sourceEntries();
  const rejected = (replacement) => assert.throws(() => assertUiBoundary(
    entries.map(([name, source]) => [name, name === "user-interface.js" ? replacement(source) : source]),
    SOURCE_MESSAGE_IDS, projectedMessages
  ), /adapter|projection|reassignment|unsupported|shadow/);
  rejected((source) => source.replace('"lsp-client-display-name"', '"lsp-client-\\x64isplay-name"'));
  rejected((source) => source.replace('output.appendLine(clientMessage(api, "lsp-client-action-stale"))',
    'const emit = clientMessage; output.appendLine(emit(api, "lsp-client-action-stale"))'));
  rejected((source) => source.replace('const output = api.window.createOutputChannel(',
    'let output; output = api.window.createOutputChannel('));
  rejected((source) => source.replace('serverStderr(message)', 'serverStderr(clientMessage)'));
  rejected((source) => source.replace('output.appendLine(clientMessage(api, "lsp-client-action-stale"))',
    'output.appendLine(clientMessage(api, condition ? "lsp-client-action-stale" : "lsp-client-action-stale"))'));
});

async function sourceEntries() {
  return Promise.all((await readdir(path.join(packageRoot, "src")))
    .filter((name) => name.endsWith(".js"))
    .map(async (name) => [name, await readFile(path.join(packageRoot, "src", name), "utf8")]));
}

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
