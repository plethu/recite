import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { CommandRegistry } from "../src/commands.js";

const cliBinary = process.env.RECITE_CLI_BIN;

test("a real recite CLI watch starts, reports a build, and cooperatively stops", {
  skip: process.platform !== "linux" || cliBinary === undefined,
  timeout: 15_000
}, async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "recite-vscode-watch-"));
  await mkdir(path.join(root, "dialogue"));
  await writeFile(path.join(root, "recite.project.toml"), `format_version = 1

[discovery]
source_roots = ["dialogue"]

[[scenes]]
id = "scene.start"
asset = "compiled/dialogue.recitec"
block = "start"
participants = ["hazel"]
`);
  await writeFile(path.join(root, "dialogue", "main.recite"),
    ":: start default speaker=hazel\n> intro@11111111111111111111\n  Hello.\n-> END\n");

  const messages = [];
  const api = hostApi(root, messages);
  const registry = new CommandRegistry(api, userInterface(messages), {
    makeInvocationId: () => "real-watch-id",
    watchStopTimeoutMs: 5_000
  });
  registry.register([]);
  try {
    const started = await api.commands.executeCommand("recite.watch.start");
    assert.deepEqual(started, { invocationId: "real-watch-id" });
    await waitFor(() => messages.some(([kind, value]) => kind === "watch" &&
      JSON.parse(value).status === "succeeded"));

    const stopped = await api.commands.executeCommand("recite.watch.stop");
    assert.deepEqual(stopped, { stopped: true, exitCode: 0 });
    assert.equal(registry.watch.active, undefined);
    assert.ok(messages.some(([kind, value]) => kind === "watch" &&
      JSON.parse(value).reason?.type === "cancelled"));
  } finally {
    await registry.dispose();
    await rm(root, { recursive: true, force: true });
  }
});

async function waitFor(predicate) {
  const deadline = Date.now() + 10_000;
  while (!predicate()) {
    if (Date.now() >= deadline) throw new Error("timed out waiting for real watch output");
    await new Promise((resolve) => setTimeout(resolve, 25));
  }
}

function hostApi(root, messages) {
  const commands = new Map();
  const entries = new Map();
  return {
    workspace: {
      isTrusted: true,
      workspaceFolders: [{ uri: { fsPath: root } }],
      textDocuments: [],
      getConfiguration: () => ({ get: (key, fallback) => ({
        "cli.path": cliBinary, "lsp.projectRoot": ""
      }[key] ?? fallback) })
    },
    window: {},
    languages: { createDiagnosticCollection: () => ({
      set: (batch) => {
        entries.clear();
        for (const [uri, values] of batch) entries.set(uri.toString(), values);
      },
      clear: () => entries.clear(), dispose() {}
    }) },
    commands: {
      registerCommand: (id, callback) => {
        commands.set(id, callback);
        return { dispose: () => commands.delete(id) };
      },
      executeCommand: (id, ...args) => commands.get(id)?.(...args)
    },
    Uri: { file: (fsPath) => ({ fsPath, toString: () => `file://${fsPath}` }) },
    Position: class Position { constructor(line, character) { this.line = line; this.character = character; } },
    Range: class Range { constructor(start, end) { this.start = start; this.end = end; } },
    Diagnostic: class Diagnostic { constructor(range, message, severity) { Object.assign(this, { range, message, severity }); } },
    DiagnosticSeverity: { Error: "error", Warning: "warning", Information: "info", Hint: "hint" }
  };
}

function userInterface(messages) {
  return {
    activeDocument: () => undefined, documentIsOpen: () => true,
    commandNotTrusted() {},
    commandDocumentRequired: () => new Error("document"),
    commandDocumentUnsaved: () => new Error("unsaved"),
    commandUntitledDocument: () => new Error("untitled"),
    commandDocumentChanged: () => new Error("changed"),
    commandDocumentOutsideRoot: () => new Error("outside"),
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
