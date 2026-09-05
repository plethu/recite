import test from "node:test";
import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { accessSync } from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import * as path from "node:path";
import { CommandRegistry } from "../src/commands.js";
const packageRoot = path.resolve(import.meta.dirname, "..");
const cliBinary = process.env.RECITE_CLI_BIN ?? path.resolve(packageRoot, "../../target/debug/recite");

test("the integrated gate supplies an executable CLI", { skip: process.env.RECITE_CLI_BIN === undefined }, () => {
  assert.ok(exists(cliBinary), `RECITE_CLI_BIN is not executable: ${cliBinary}`);
});

test("registered validate command resolves argv and cwd, maps diagnostics, and refuses untrusted workspaces", async () => {
  const calls = [];
  const messages = [];
  const api = hostApi({ trusted: true, messages, document: savedDocument(), documents: [
    savedDocumentAt("/workspace/project/dialogue.recite"),
    savedDocumentAt("/workspace/project/first.recite"), savedDocumentAt("/workspace/project/second.recite")
  ] });
  const registry = new CommandRegistry(api, userInterface(messages), {
    makeInvocationId: () => "validate-id",
    spawnProcess: (_command, args, options) => {
      calls.push({ args, options });
      const child = new FakeChild();
      queueMicrotask(() => {
        child.stdout.emit("data", Buffer.from(JSON.stringify({
          version: 1, sequence: 0, event: "command.started", command: "validate", invocation_id: "validate-id"
        }) + "\n" + JSON.stringify({
          version: 1, sequence: 1, event: "command.result", command: "validate", invocation_id: "validate-id",
          status: "success", exit_code: 0, data: { diagnostics: [{
            version: 1, code: "RECITE_PARSE001", severity: "error", span: {
              file: "dialogue.recite", start: { line: 1, column: 1 }, end: { line: 1, column: 3 }
            }, presentation: { id: "diagnostic-parse-001", arguments: {} }, related: [], help: null,
            explanation: null, compatibility_message: "diagnostic"
          }] }
        }) + "\n"));
        child.close(0);
      });
      return child;
    }
  });
  registry.register([]);
  await api.commands.executeCommand("recite.validate");
  assert.deepEqual(calls[0].args, [
    "validate", "/workspace/project/dialogue.recite", "--output-format", "structured", "--invocation-id", "validate-id"
  ]);
  assert.equal(calls[0].options.cwd, "/workspace/project");
  assert.equal(calls[0].options.shell, false);
  assert.equal(api.cliDiagnostics.entries.get("file:///workspace/project/dialogue.recite").length, 1);
  assert.equal(messages.at(-1)[0], "result");

  api.workspace.isTrusted = false;
  await api.commands.executeCommand("recite.validate");
  assert.equal(messages.at(-1)[0], "untrusted");
});
test("typed finite failures stay typed at the semantic UI boundary", async () => {
  const messages = [];
  const api = hostApi({ trusted: true, messages, document: savedDocument() });
  const registry = new CommandRegistry(api, userInterface(messages), {
    makeInvocationId: () => "failure-id",
    spawnProcess: () => {
      const child = new FakeChild();
      queueMicrotask(() => {
        child.stdout.emit("data", Buffer.from(JSON.stringify({
          version: 1, sequence: 0, event: "command.started", command: "validate", invocation_id: "failure-id"
        }) + "\n" + JSON.stringify({
          version: 1, sequence: 1, event: "command.error", command: "validate", invocation_id: "failure-id",
          status: "failure", exit_code: 1,
          error: { category: "input", code: "read", operation: "validate" }
        }) + "\n"));
        child.close(1);
      });
      return child;
    }
  });
  registry.register([]);
  const result = await api.commands.executeCommand("recite.validate");
  assert.equal(result.terminal.event, "command.error");
  assert.equal(messages.at(-1)[0], "failure");
});

test("validate refuses a saved document outside the effective project root", async () => {
  const messages = [];
  const document = savedDocumentAt("/other/dialogue.recite");
  const api = hostApi({ trusted: true, messages, document, documents: [document] });
  let spawned = false;
  const registry = new CommandRegistry(api, userInterface(messages, document), {
    makeInvocationId: () => "outside-id",
    spawnProcess: () => { spawned = true; return new FakeChild(); }
  });
  registry.register([]);
  assert.equal(await api.commands.executeCommand("recite.validate"), undefined);
  assert.equal(spawned, false);
  assert.equal(messages.at(-1)[0], "failure");
});

test("compile revalidates the saved document after its output picker", async () => {
  const messages = [];
  let resolveOutput;
  let spawned = false;
  const api = hostApi({ trusted: true, messages, document: savedDocument() });
  const ui = userInterface(messages, api.window.activeTextEditor.document);
  ui.chooseCompileOutputPath = () => new Promise((resolve) => { resolveOutput = resolve; });
  const registry = new CommandRegistry(api, ui, {
    makeInvocationId: () => "changed-id",
    spawnProcess: () => { spawned = true; return new FakeChild(); }
  });
  registry.register([]);
  const pending = api.commands.executeCommand("recite.compile");
  assert.equal(typeof resolveOutput, "function");
  api.window.activeTextEditor.document.version = 2;
  api.window.activeTextEditor.document.isDirty = true;
  resolveOutput("/workspace/project/out.recitec");
  assert.equal(await pending, undefined);
  assert.equal(spawned, false);
  assert.equal(messages.at(-1)[0], "failure");
});

test("extract picker cancellation does not launch the stdout form", async () => {
  const messages = [];
  const api = hostApi({ trusted: true, messages, document: savedDocument() });
  const ui = userInterface(messages, api.window.activeTextEditor.document);
  ui.chooseExtractOutputPath = async () => undefined;
  let spawned = false;
  const registry = new CommandRegistry(api, ui, {
    spawnProcess: () => { spawned = true; return new FakeChild(); }
  });
  registry.register([]);
  assert.equal(await api.commands.executeCommand("recite.extract"), undefined);
  assert.equal(spawned, false);
  assert.deepEqual(messages, []);
});

test("compile refuses a configuration authority change across an output picker", async () => {
  const messages = [];
  let resolveOutput;
  let projectRoot = "project";
  let call;
  const api = hostApi({ trusted: true, messages, document: savedDocument() });
  api.workspace.getConfiguration = () => ({ get: (key, fallback) => ({
    "cli.path": "recite", "lsp.projectRoot": projectRoot
  }[key] ?? fallback) });
  const ui = userInterface(messages, api.window.activeTextEditor.document);
  ui.chooseCompileOutputPath = () => new Promise((resolve) => { resolveOutput = resolve; });
  const registry = new CommandRegistry(api, ui, {
    makeInvocationId: () => "snapshot-id",
    spawnProcess: (_command, args, options) => {
      call = { args, options };
      const child = new FakeChild();
      queueMicrotask(() => {
        child.stdout.emit("data", Buffer.from(JSON.stringify({
          version: 1, sequence: 0, event: "command.started", command: "compile", invocation_id: "snapshot-id"
        }) + "\n" + JSON.stringify({
          version: 1, sequence: 1, event: "command.result", command: "compile", invocation_id: "snapshot-id",
          status: "success", exit_code: 0, data: {
            diagnostics: [], artifact: {
              path: { encoding: "utf8", value: "/workspace/project/out.recitec" }, size_bytes: 0
            }
          }
        }) + "\n"));
        child.close(0);
      });
      return child;
    }
  });
  registry.register([]);
  const pending = api.commands.executeCommand("recite.compile");
  assert.equal(typeof resolveOutput, "function");
  projectRoot = "other";
  resolveOutput("/workspace/project/out.recitec");
  assert.equal(await pending, undefined);
  assert.equal(call, undefined);
});

test("runtime picker trust loss is a quiet refusal before spawn", async () => {
  const messages = [];
  let resolveAsset;
  const api = hostApi({ trusted: true, messages, document: savedDocument() });
  const ui = userInterface(messages);
  ui.chooseAssetPath = () => new Promise((resolve) => { resolveAsset = resolve; });
  ui.chooseBlock = async () => "start";
  ui.chooseFixturePath = async () => ["/workspace/project/fixture.toml"];
  let spawned = false;
  const registry = new CommandRegistry(api, ui, {
    makeInvocationId: () => "trust-id",
    spawnProcess: () => { spawned = true; return new FakeChild(); }
  });
  registry.register([]);
  const pending = api.commands.executeCommand("recite.run");
  api.workspace.isTrusted = false;
  resolveAsset(["/workspace/project/asset.recitec"]);
  assert.equal(await pending, undefined);
  assert.equal(spawned, false);
  assert.deepEqual(messages, [["untrusted"]]);
});
test("a built recite CLI speaks the finite protocol through the command adapter", {
  skip: process.platform !== "linux" || process.env.RECITE_CLI_BIN === undefined,
  timeout: 15_000
}, async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "recite-vscode-cli-"));
  const sourcePath = path.join(root, "dialogue.recite");
  await writeFile(sourcePath,
    ":: start default\n> line@11111111111111111111\n  Hello.\n-> END\n");
  const messages = [];
  const document = {
    languageId: "recite", isUntitled: false, isDirty: false,
    uri: { scheme: "file", fsPath: sourcePath, toString: () => `file://${sourcePath}` }
  };
  const api = hostApi({
    trusted: true, messages, document, root, projectRoot: "", cliPath: cliBinary
  });
  const registry = new CommandRegistry(api, userInterface(messages, document), {
    makeInvocationId: () => "real-cli-id"
  });
  registry.register([]);
  try {
    const result = await api.commands.executeCommand("recite.validate");
    assert.equal(result.terminal.event, "command.result");
    assert.equal(result.terminal.status, "success");
    assert.deepEqual(result.terminal.data.diagnostics, []);
  } finally {
    await registry.dispose();
    await rm(root, { recursive: true, force: true });
  }
});

class FakeChild extends EventEmitter {
  constructor() {
    super();
    this.stdout = new EventEmitter();
    this.stderr = new EventEmitter();
    this.stdin = new EventEmitter();
    this.stdin.writable = true;
    this.stdin.writes = [];
    this.stdin.write = (value) => {
      this.stdin.writes.push(JSON.parse(value));
      return true;
    };
    this.stdin.end = () => {};
    this.stdin.destroy = () => { this.stdin.writable = false; };
    this.killed = false;
  }

  kill(signal) {
    this.killed = true;
    if (signal === "SIGKILL") queueMicrotask(() => this.close(1));
  }
  close(code) { this.emit("close", code, null); }
}

function savedDocument() {
  return savedDocumentAt("/workspace/project/dialogue.recite");
}

function savedDocumentAt(fsPath) {
  return {
    languageId: "recite", isUntitled: false, isDirty: false, version: 1,
    uri: { scheme: "file", fsPath, toString: () => `file://${fsPath}` },
    getText: () => ":: start"
  };
}

function hostApi({ trusted, messages, document, documents = document ? [document] : [], root = "/workspace", projectRoot = "project",
  cliPath = "./tools/recite" }) {
  const commands = new Map();
  const entries = new Map();
  const api = {
    workspace: {
      isTrusted: trusted,
      workspaceFolders: [{ uri: { fsPath: root, toString: () => `file://${root}` } }],
      textDocuments: documents,
      getConfiguration: () => ({ get: (key, fallback) => ({ "cli.path": cliPath, "lsp.projectRoot": projectRoot }[key] ?? fallback) })
    },
    window: { activeTextEditor: { document }, showSaveDialog: async () => undefined },
    languages: { createDiagnosticCollection: () => ({
      entries,
      clear: () => entries.clear(),
      set: (batch) => {
        entries.clear();
        for (const [uri, values] of batch) entries.set(uri.toString(), values);
      },
      dispose() {}
    }) },
    commands: {
      registerCommand: (id, callback) => { commands.set(id, callback); return { dispose: () => commands.delete(id) }; },
      executeCommand: (id, ...args) => commands.get(id)?.(...args)
    },
    Uri: { file: (fsPath) => ({ fsPath, toString: () => `file://${fsPath}` }) },
    Position: class Position { constructor(line, character) { this.line = line; this.character = character; } },
    Range: class Range { constructor(start, end) { this.start = start; this.end = end; } },
    Diagnostic: class Diagnostic { constructor(range, message, severity) { Object.assign(this, { range, message, severity }); } },
    DiagnosticSeverity: { Error: "error", Warning: "warning", Information: "info", Hint: "hint" },
    cliDiagnostics: { entries }
  };
  Object.defineProperty(api.workspace, "isTrusted", { get: () => trusted, set: (value) => { trusted = value; } });
  return api;
}

function exists(file) {
  try {
    accessSync(file);
    return true;
  } catch {
    return false;
  }
}

function userInterface(messages, document = savedDocument()) {
  return {
    activeDocument: () => document,
    documentIsOpen: () => true,
    chooseCompileOutputPath: async () => undefined,
    chooseExtractOutputPath: async () => undefined,
    chooseAssetPath: async () => undefined,
    chooseBlock: async () => undefined,
    chooseFixturePath: async () => undefined,
    commandNotTrusted: () => messages.push(["untrusted"]),
    commandDocumentRequired: () => new Error("document"),
    commandDocumentUnsaved: () => new Error("unsaved"),
    commandUntitledDocument: () => new Error("untitled"),
    commandDocumentChanged: () => new Error("changed"),
    commandDocumentOutsideRoot: () => new Error("outside-root"),
    commandWorkspaceRequired: () => new Error("workspace"),
    cliPathInvalid: () => new Error("path"),
    commandInputInvalid: () => new Error("input"),
    commandWatchRunning: () => messages.push(["running"]),
    commandWatchNotRunning: () => messages.push(["not-running"]),
    commandWatchStopTimeout: () => messages.push(["timeout"]),
    commandResult: (value) => messages.push(["result", value]),
    commandContentDiagnostics: (value) => messages.push(["diagnostics", value]),
    commandFailure: (value) => messages.push(["failure", value]),
    commandProtocolFailure: (value) => messages.push(["protocol", value]),
    commandWatchStatus: (value) => messages.push(["watch", value])
  };
}
