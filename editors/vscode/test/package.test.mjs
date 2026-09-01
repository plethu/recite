import test from "node:test";
import assert from "node:assert/strict";
import { readFile, writeFile, readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { mkdtemp, mkdir, rm, symlink } from "node:fs/promises";
import os from "node:os";
import { assertContainedRegularFile, assertSafeTree } from "../scripts/safety.mjs";
import { assertSourceMessageOwnership } from "../scripts/source-messages.mjs";
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

test("source message ownership rejects literals and comment/string decoys", async () => {
  const entries = await Promise.all((await readdir(path.join(packageRoot, "src")))
    .filter((name) => name.endsWith(".js") && name !== "messages.js")
    .map(async (name) => [name, await readFile(path.join(packageRoot, "src", name), "utf8")]));
  assert.doesNotThrow(() => assertSourceMessageOwnership(
    entries, SOURCE_MESSAGE_IDS, projectedMessages
  ));
  const hostile = entries.map(([name, source]) => [name, name === "edit-commands.js"
    ? source.replace('clientMessage(this.api, "lsp-client-action-stale")',
      '// clientMessage(this.api, "lsp-client-action-stale")\n'
      + '/* clientMessage(this.api, "lsp-client-action-stale") */\n'
      + '"clientMessage(this.api, \\\"lsp-client-action-stale\\\")"')
    : source]);
  assert.throws(() => assertSourceMessageOwnership(
    hostile, SOURCE_MESSAGE_IDS, projectedMessages
  ), /source message use must cover exactly the owned IDs|every clientMessage call|visible host output/);
});

test("every visible host output boundary rejects a newly added literal", async () => {
  const entries = await Promise.all((await readdir(path.join(packageRoot, "src")))
    .filter((name) => name.endsWith(".js") && name !== "messages.js")
    .map(async (name) => [name, await readFile(path.join(packageRoot, "src", name), "utf8")]));
  const injections = [
    ["controller.js", "this.output.append(message)", "this.output.append(\"English literal\")"],
    ["controller.js", "clientMessage(this.api, \"lsp-client-start-failed\", error.message)", "\"English literal\""],
    ["extension.js", "clientMessage(vscode, \"lsp-client-display-name\")", "\"English literal\""],
    ["edit-commands.js", "clientMessage(this.api, \"lsp-client-action-stale\")", "\"English literal\""]
  ];
  for (const [file, original, replacement] of injections) {
    const hostile = entries.map(([name, source]) => [name, name === file
      ? source.replace(original, replacement)
      : source]);
    assert.throws(() => assertSourceMessageOwnership(
      hostile, SOURCE_MESSAGE_IDS, projectedMessages
    ), /visible host output/);
  }
});

test("source ownership is syntax-aware across hostile JavaScript forms", () => {
  const valid = (source) => assert.doesNotThrow(() => assertSourceMessageOwnership(
    [["hostile.js", source]], ["lsp-client-start-failed"], projectedMessages
  ));
  const rejected = (source, expected = /visible host output|canonical wrapper|message use/) =>
    assert.throws(() => assertSourceMessageOwnership(
      [["hostile.js", source]], ["lsp-client-start-failed"], projectedMessages
    ), expected);

  rejected(String.raw`const decoy = /clientMessage(api, "lsp-client-start-failed")/;`);
  rejected(String.raw`const output = {}; output["appendLine"]("English literal");`);
  rejected(String.raw`const output = {}; const method = "appendLine"; output[method]("English literal");`);
  rejected(String.raw`const output = {}; const emit = output.appendLine; emit("English literal");`);
  rejected(String.raw`const output = {}; const { appendLine: emit } = output; emit("English literal");`);
  rejected(String.raw`const output = {}; const emit = output.appendLine.bind(output); emit("English literal");`);
  rejected(String.raw`const output = {}; const text = "English literal"; output.appendLine(text);`);
  rejected([
    "const output = {};",
    "output.appendLine(`prefix ${detail}`);"
  ].join("\n"));
  rejected([
    "const output = {};",
    "output.appendLine(`${\"English literal\"}`);"
  ].join("\n"));
  rejected(String.raw`const output = {}; output.appendLine("prefix " + detail);`);
  for (const method of ["showWarningMessage", "showErrorMessage", "showInformationMessage"]) {
    rejected(`const api = { window: { ${method}() {} } }; api.window.${method}("literal");`);
  }
  rejected(String.raw`const api = { window: { showInformationMessage() {} } };
    const { showInformationMessage } = api.window;
    showInformationMessage("Information");`);

  valid([
    'import { clientMessage as message } from "./messages.js";',
    'const output = { appendLine() {} };',
    'output.appendLine(`${message(api, "lsp-client-start-failed", detail)}`);'
  ].join("\n"));
  valid(String.raw`import { clientMessage } from "./messages.js";
    const output = { appendLine() {} };
    output.appendLine(clientMessage(api, "lsp-client-\x73tart-failed", detail));`);
  valid(String.raw`import * as messages from "./messages.js";
    const output = { appendLine() {} };
    const { clientMessage: emit } = messages;
    output.appendLine(emit(api, "lsp-client-start-failed"));`);
  valid(String.raw`import * as messages from "./messages.js";
    const output = { appendLine() {} };
    const emit = messages["clientMessage"];
    output.appendLine(emit(api, "lsp-client-start-failed"));`);
  valid(String.raw`// clientMessage(api, "lsp-client-start-failed")
    const text = "clientMessage(api, lsp-client-start-failed)";
    const regular = /clientMessage\(api, "lsp-client-start-failed"\)/;
    const panel = { render() {} };
    panel.render("English literal");
    clientMessage(api, "lsp-client-start-failed");`);
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
