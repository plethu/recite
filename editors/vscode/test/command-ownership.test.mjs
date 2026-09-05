import test from "node:test";
import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { CommandRegistry } from "../src/commands.js";

test("run and trace preserve the diagnostic snapshot", async () => {
  const h = harness();
  h.entries.set("seed", ["existing"]);
  const registry = new CommandRegistry(h.api, h.ui, {
    makeInvocationId: () => "run-id",
    spawnProcess: (_command, args) => finiteChild("run", "run-id", {
      trace: { asset_id: "asset", block: "start", events: [], final_deferred_effects: [] }
    }, args)
  });
  registry.register([]);
  await h.api.commands.executeCommand("recite.run", {
    asset: "/workspace/project/asset.recitec", block: "start", fixture: "/workspace/project/fixture.toml"
  });
  assert.deepEqual(h.entries.get("seed"), ["existing"]);
});

test("only the latest finite invocation may replace diagnostics", async () => {
  const h = harness();
  const children = [];
  const registry = new CommandRegistry(h.api, h.ui, {
    makeInvocationId: () => `id-${children.length}`,
    spawnProcess: () => {
      const child = new DeferredChild();
      children.push(child);
      return child;
    }
  });
  registry.register([]);
  const first = h.api.commands.executeCommand("recite.validate");
  const second = h.api.commands.executeCommand("recite.validate");
  children[1].finish("validate", "id-1", diagnosticResult("second.recite"), 0);
  await second;
  children[0].finish("validate", "id-0", diagnosticResult("first.recite"), 0);
  await first;
  assert.equal(h.entries.has("file:///workspace/project/second.recite"), true);
  assert.equal(h.entries.has("file:///workspace/project/first.recite"), false);
});

test("a valid terminal with an unprojectable diagnostic preserves the prior snapshot", async () => {
  const h = harness();
  h.entries.set("seed", ["existing"]);
  const registry = new CommandRegistry(h.api, h.ui, {
    makeInvocationId: () => "atomic-id",
    spawnProcess: () => {
      const child = new DeferredChild();
      queueMicrotask(() => child.finish("validate", "atomic-id", diagnosticResult("missing.recite"), 0));
      return child;
    }
  });
  registry.register([]);
  assert.equal(await h.api.commands.executeCommand("recite.validate"), undefined);
  assert.deepEqual([...h.entries], [["seed", ["existing"]]]);
  assert.equal(h.messages.at(-1)[0], "protocol");
});

test("a document mutation before close retires the result without publishing it", async () => {
  const h = harness();
  let child;
  const registry = new CommandRegistry(h.api, h.ui, {
    makeInvocationId: () => "stale-id",
    spawnProcess: () => (child = new DeferredChild())
  });
  registry.register([]);
  const validation = h.api.commands.executeCommand("recite.validate");
  h.document.version = 2;
  h.document.isDirty = true;
  child.finish("validate", "stale-id", diagnosticResult("first.recite"), 0);
  assert.equal(await validation, undefined);
  assert.equal(h.entries.size, 0);
});

test("switching tabs does not stale an unchanged captured source document", async () => {
  const h = harness();
  let child;
  const other = {
    ...h.document,
    uri: { scheme: "file", fsPath: "/workspace/project/other.recite", toString: () => "file:///workspace/project/other.recite" }
  };
  let active = h.document;
  h.ui.activeDocument = () => active;
  const registry = new CommandRegistry(h.api, h.ui, {
    makeInvocationId: () => "tab-switch-id",
    spawnProcess: () => (child = new DeferredChild())
  });
  registry.register([]);
  const validation = h.api.commands.executeCommand("recite.validate");
  active = other;
  child.finish("validate", "tab-switch-id", diagnosticResult("first.recite"), 0);
  assert.equal((await validation).terminal.status, "success");
  assert.equal(h.entries.has("file:///workspace/project/first.recite"), true);
});

test("dispose force-closes an in-flight finite child and suppresses late UI", async () => {
  const h = harness();
  let child;
  const registry = new CommandRegistry(h.api, h.ui, {
    makeInvocationId: () => "dispose-id",
    spawnProcess: () => (child = new DeferredChild()), disposeTimeoutMs: 5, forceDisposeTimeoutMs: 20
  });
  registry.register([]);
  const validation = h.api.commands.executeCommand("recite.validate");
  await registry.dispose();
  assert.deepEqual(child.signals, ["SIGTERM", "SIGKILL"]);
  child.finish("validate", "dispose-id", diagnosticResult("first.recite"), 0);
  assert.equal(await validation, undefined);
  assert.equal(h.messages.length, 0);
});

test("commands invoked after disposal never spawn or publish", async () => {
  const h = harness();
  let spawned = 0;
  const registry = new CommandRegistry(h.api, h.ui, {
    spawnProcess: () => { spawned += 1; return new DeferredChild(); }
  });
  registry.register([]);
  await registry.dispose();
  assert.equal(await h.api.commands.executeCommand("recite.validate"), undefined);
  assert.equal(spawned, 0);
  assert.equal(h.messages.length, 0);
});

test("authority transition retires finite children before late completion", async () => {
  const h = harness();
  let child;
  const registry = new CommandRegistry(h.api, h.ui, {
    makeInvocationId: () => "authority-id",
    spawnProcess: () => (child = new DeferredChild()),
    authorityStopTimeoutMs: 5,
    authorityForceStopTimeoutMs: 20
  });
  registry.register([]);
  const validation = h.api.commands.executeCommand("recite.validate");
  await registry.stopForAuthorityChange();
  assert.deepEqual(child.signals, ["SIGTERM", "SIGKILL"]);
  child.finish("validate", "authority-id", diagnosticResult("first.recite"), 0);
  assert.equal(await validation, undefined);
  assert.equal(h.messages.length, 0);
  assert.equal(h.entries.size, 0);
});

function diagnosticResult(file) {
  return {
    status: "success", exit_code: 0,
    data: { diagnostics: [{
      version: 1, code: "RECITE_PARSE001", severity: "error",
      span: { file, start: { line: 1, column: 1 }, end: { line: 1, column: 1 } },
      presentation: { id: "diagnostic-parse-001", arguments: {} }, related: [], help: null,
      explanation: null, compatibility_message: "fallback"
    }] }
  };
}

function finiteChild(command, invocationId, data, args) {
  const child = new DeferredChild();
  queueMicrotask(() => child.finish(command, invocationId, { status: "success", exit_code: 0, data }, 0));
  return child;
}

function harness() {
  const entries = new Map();
  const messages = [];
  const document = {
    languageId: "recite", isUntitled: false, isDirty: false, version: 1,
    uri: { scheme: "file", fsPath: "/workspace/project/dialogue.recite", toString: () => "file:///workspace/project/dialogue.recite" },
    getText: () => ":: start"
  };
  const docs = [document, ...["first.recite", "second.recite"].map((file) => ({
    ...document, uri: { scheme: "file", fsPath: `/workspace/project/${file}`, toString: () => `file:///workspace/project/${file}` }
  }))];
  const commands = new Map();
  const api = {
    workspace: {
      isTrusted: true, workspaceFolders: [{ uri: { fsPath: "/workspace" } }], textDocuments: docs,
      getConfiguration: () => ({ get: (key, fallback) => ({ "cli.path": "recite", "lsp.projectRoot": "project" }[key] ?? fallback) })
    },
    window: { activeTextEditor: { document } },
    languages: { createDiagnosticCollection: () => ({
      entries,
      clear: () => entries.clear(),
      set: (batch) => {
        entries.clear();
        for (const [uri, values] of batch) entries.set(uri.toString(), values);
      },
      dispose() {}
    }) },
    commands: { registerCommand: (id, cb) => { commands.set(id, cb); return { dispose() {} }; }, executeCommand: (id, ...args) => commands.get(id)?.(...args) },
    Uri: { file: (fsPath) => ({ fsPath, toString: () => `file://${fsPath}` }) },
    Position: class Position { constructor(line, character) { this.line = line; this.character = character; } },
    Range: class Range { constructor(start, end) { this.start = start; this.end = end; } },
    Diagnostic: class Diagnostic { constructor(range, message, severity) { Object.assign(this, { range, message, severity }); } },
    DiagnosticSeverity: { Error: "error", Warning: "warning", Information: "info", Hint: "hint" }
  };
  return { api, entries, messages, document, ui: ui(messages, document) };
}

function ui(messages, document) {
  return {
    activeDocument: () => document,
    documentIsOpen: () => true,
    commandNotTrusted() {}, commandDocumentRequired: () => new Error("document"), commandDocumentUnsaved: () => new Error("unsaved"),
    commandUntitledDocument: () => new Error("untitled"), commandDocumentChanged: () => new Error("changed"),
    commandDocumentOutsideRoot: () => new Error("outside"), commandWorkspaceRequired: () => new Error("workspace"),
    cliPathInvalid: () => new Error("path"), commandInputInvalid: () => new Error("input"),
    commandWatchRunning() {}, commandWatchNotRunning() {}, commandWatchStopTimeout() {},
    commandResult: (value) => messages.push(["result", value]), commandContentDiagnostics: (value) => messages.push(["diagnostics", value]),
    commandFailure: (value) => messages.push(["failure", value]), commandProtocolFailure: (value) => messages.push(["protocol", value]), commandWatchStatus() {},
    chooseCompileOutputPath: async () => undefined, chooseExtractOutputPath: async () => undefined,
    chooseAssetPath: async () => undefined, chooseBlock: async () => undefined, chooseFixturePath: async () => undefined
  };
}

class DeferredChild extends EventEmitter {
  constructor() {
    super(); this.stdout = new EventEmitter(); this.stderr = new EventEmitter(); this.signals = [];
    this.stdin = new EventEmitter(); this.stdin.writable = true; this.stdin.end = () => {}; this.stdin.destroy = () => { this.stdin.writable = false; };
  }
  kill(signal) { this.signals.push(signal); if (signal === "SIGKILL") this.close(1); }
  finish(command, invocationId, terminal, code) {
    this.stdout.emit("data", Buffer.from(JSON.stringify({ version: 1, sequence: 0, event: "command.started", command, invocation_id: invocationId }) + "\n" +
      JSON.stringify({ version: 1, sequence: 1, event: "command.result", command, invocation_id: invocationId, ...terminal }) + "\n"));
    this.close(code);
  }
  close(code) { this.emit("close", code, null); }
}
