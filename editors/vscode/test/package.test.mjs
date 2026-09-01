import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, symlink, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import os from "node:os";
import { assertContainedRegularFile, assertSafeTree } from "../scripts/safety.mjs";
import { listSourceModules } from "../scripts/source-files.mjs";
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

test("nested JavaScript modules are enumerated safely and reach the UI boundary", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "recite-vscode-source-"));
  try {
    await mkdir(path.join(root, "nested"));
    await writeFile(path.join(root, "top.js"), "const top = true;\n");
    for (const extension of [".js", ".mjs", ".cjs"]) {
      await writeFile(path.join(root, "nested", `unrelated${extension}`),
        "const host = { appendLine() {} }; host.appendLine(value);\n");
    }
    assert.deepEqual(listSourceModules(root).map(({ relativePath }) => relativePath), [
      "nested/unrelated.cjs", "nested/unrelated.js", "nested/unrelated.mjs", "top.js"
    ]);
    const entries = await sourceEntries(root);
    const packageEntries = await sourceEntries();
    assert.doesNotThrow(() => assertUiBoundary(
      [
        ...packageEntries,
        ...entries
      ], SOURCE_MESSAGE_IDS, projectedMessages
    ));

    for (const extension of [".mjs", ".cjs"]) {
      await writeFile(path.join(root, "nested", `hostile${extension}`),
        "const output = {}; output.appendLine(\"English\");\n");
      const hostileEntries = await sourceEntries(root);
      assert.throws(() => assertUiBoundary(
        [...packageEntries, ...hostileEntries],
        SOURCE_MESSAGE_IDS, projectedMessages
      ), /outside|acquisition|access|call|projection/);
    }

    await symlink(path.join(root, "top.js"), path.join(root, "nested", "escape.js"));
    assert.throws(() => listSourceModules(root), /symlink/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
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
  ), /outside|acquisition|access|import|load|call|projection|re-export/);
  for (const source of [
    String.raw`function fake(clientMessage) { clientMessage(api, "lsp-client-start-failed"); }`,
    String.raw`function outer() { function fake(clientMessage) { return clientMessage(api, id); } }`,
    String.raw`const clientMessage = () => {}; clientMessage(api, "lsp-client-start-failed");`,
    String.raw`let emit; emit = clientMessage;`,
    String.raw`let emit; emit = output.appendLine; emit(value);`,
    String.raw`const output = {}; output.appendLine("English");`,
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
    String.raw`const ui = createUserInterface(api); ui.serverError("English");`,
    String.raw`const ui = createUserInterface(api); ui.serverError(` + "`English ${detail}`" + String.raw`);`,
    String.raw`const ui = createUserInterface(api); ui.serverError(prefix + detail);`,
    String.raw`const ui = createUserInterface(api); ui.serverError(condition ? detail : other);`,
    String.raw`const ui = createUserInterface(api); ui.serverError(format(detail));`,
    String.raw`const ui = createUserInterface(api); ui.serverError(...[detail]);`,
    String.raw`const ui = createUserInterface(api); ui.actionStale(detail);`,
    String.raw`const ui = createUserInterface(api); ui.serverStderr("English");`,
    String.raw`const ui = createUserInterface(api); ui.serverNotification(prefix + message);`,
    String.raw`const ui = createUserInterface(api); ui.serverStderr.call(ui, message);`,
    String.raw`const ui = createUserInterface(api); ui.serverNotification.apply(ui, [message]);`,
    String.raw`const api = { window: {} }; api.window.showWarningMessage("Warning");`,
    String.raw`const api = { window: {} }; api.window.showErrorMessage("Error");`,
    String.raw`const api = { window: {} }; api.window.showInformationMessage("Information");`,
    String.raw`const api = { window: {} }; api.window[method]("Warning");`,
    String.raw`const ui = {}; ui[method]("Warning");`,
    String.raw`const controller = { userInterface: {} }; controller.userInterface[method]("Warning");`,
    String.raw`const ui = createUserInterface(api); ui[method](detail);`,
    String.raw`const ui = createUserInterface(api); Reflect.get(ui, method);`,
    String.raw`const ui = createUserInterface(api); Reflect.apply(ui.serverError, ui, [detail]);`,
    String.raw`Reflect.get(api.window, method);`,
    String.raw`Reflect.get(api, "window");`,
    String.raw`Reflect["get"](api.window, method);`,
    String.raw`Reflect[reflectMethod](api.window, method);`,
    String.raw`Reflect.apply(vscode.window.showErrorMessage, vscode.window, [detail]);`,
    String.raw`import("./messages.js");`,
    String.raw`import("./user-interface.js");`,
    String.raw`export * from "./messages.js";`,
    String.raw`export * as messages from "./messages.js";`,
    String.raw`import(` + "`./messages.js`" + String.raw`);`,
    String.raw`require("./messages.js");`,
    String.raw`const { appendLine: emit, ...rest } = output;`,
    String.raw`const { window } = api; window.showErrorMessage("English");`,
    String.raw`const currentWindow = window; currentWindow[method]("English");`,
    String.raw`import { clientMessage } from "./messages.js";`,
    String.raw`const api = {}; api.window.createOutputChannel("name");`
  ]) rejected(source);
  assert.doesNotThrow(() => assertUiBoundary([
    ...entries, ["builder.js", "const builder = { append() {} }; builder.append(value);"]
  ], SOURCE_MESSAGE_IDS, projectedMessages));
  assert.doesNotThrow(() => assertUiBoundary([
    ...entries, ["unrelated-properties.js", [
      "const host = { append() {} }; host.append(value);",
      "const { appendLine: emit, ...rest } = unrelated;",
      "const copy = { appendLine };",
      "export * from \"./unrelated.js\";",
      "export * as unrelated from \"./unrelated.js\";",
      "import(`./unrelated.js`);",
      "import(`./messages${suffix}.js`);"
    ].join("\n")]
  ], SOURCE_MESSAGE_IDS, projectedMessages));
});

test("the UI service accepts only its structural caller contracts", async () => {
  const entries = await sourceEntries();
  const valid = [
    "const ui1 = createUserInterface(api); ui1.serverLifecycleFailure(detail);",
    "const ui2 = createUserInterface(api); ui2.serverExited();",
    "const ui3 = createUserInterface(api); ui3.serverStderr(message);",
    "const ui4 = createUserInterface(api); ui4.serverLogMessage(event.message);",
    "const ui5 = createUserInterface(api); ui5.actionStale();",
    "const ui6 = createUserInterface(api); ui6.dispose();",
    "const host = { appendLine() {} }; host.appendLine(value);",
    "const { appendLine, ...rest } = unrelated; const copy = { appendLine };",
    "const unrelated = { output: { appendLine() {} } }; unrelated.output.appendLine(value);"
  ];
  assert.doesNotThrow(() => assertUiBoundary(
    [...entries, ["valid-contracts.js", valid.join("\n")]],
    SOURCE_MESSAGE_IDS, projectedMessages
  ));
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
  rejected((source) => `${source}\nconst extra = 1;`);
  rejected((source) => source.replace('export function createUserInterface',
    'export async function createUserInterface'));
  rejected((source) => source.replace('serverLifecycleFailure(detail) {', 'async serverLifecycleFailure(detail) {'));
});

async function sourceEntries(sourceRoot = path.join(packageRoot, "src")) {
  return Promise.all(listSourceModules(sourceRoot).map(async ({ relativePath, absolutePath }) => [
    relativePath,
    await readFile(absolutePath, "utf8")
  ]));
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
