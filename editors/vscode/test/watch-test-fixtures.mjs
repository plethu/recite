import { EventEmitter } from "node:events";
import { CommandRegistry } from "../src/commands.js";

export function watchRegistry(messages, child, invocationId, watchStopTimeoutMs = 1_500,
  projectRoot = "", documents = []) {
  const entries = new Map();
  const commands = new Map();
  const api = {
    workspace: {
      isTrusted: true,
      workspaceFolders: [{ uri: { fsPath: "/project" } }],
      textDocuments: documents,
      getConfiguration: () => ({ get: (key, fallback) => ({ "lsp.projectRoot": projectRoot }[key] ?? fallback) })
    },
    languages: { createDiagnosticCollection: () => ({
      clear: () => entries.clear(),
      set: (batch) => {
        entries.clear();
        for (const [uri, values] of batch) entries.set(uri.toString(), values);
      },
      dispose() {}
    }) },
    commands: {
      registerCommand: (id, callback) => {
        commands.set(id, callback);
        return { dispose: () => commands.delete(id) };
      },
      executeCommand: (id, ...args) => commands.get(id)?.(...args)
    }
  };
  return new CommandRegistry(api, {
    commandNotTrusted: () => messages.push(["untrusted"]),
    commandDocumentRequired: () => new Error("document"),
    commandDocumentUnsaved: () => new Error("unsaved"),
    commandUntitledDocument: () => new Error("untitled"),
    commandWorkspaceRequired: () => new Error("workspace"),
    cliPathInvalid: () => new Error("path"),
    commandInputInvalid: () => new Error("input"),
    commandWatchRunning: () => messages.push(["running"]),
    commandWatchNotRunning: () => messages.push(["not-running"]),
    commandWatchStopTimeout: () => messages.push(["timeout"]),
    commandResult() {}, commandContentDiagnostics() {}, commandFailure: (value) => messages.push(["failure", value]),
    commandProtocolFailure: (value) => messages.push(["protocol", value]), commandWatchStatus: (value) => messages.push(["watch", value]),
    activeDocument: () => undefined, documentIsOpen: () => true, chooseCompileOutputPath: async () => undefined,
    chooseExtractOutputPath: async () => undefined, chooseAssetPath: async () => undefined,
    chooseBlock: async () => undefined, chooseFixturePath: async () => undefined
  }, {
    makeInvocationId: () => invocationId,
    spawnProcess: () => child,
    watchStopTimeoutMs,
    watchForceKillDelayMs: 10
  });
}

export function watchDiagnostic(file) {
  return {
    version: 1, code: "RECITE_TEST001", severity: "error",
    span: { file, start: { line: 1, column: 1 }, end: null },
    presentation: { id: "diagnostic-parse-001", arguments: {} }, related: [], help: null,
    explanation: null, compatibility_message: "fallback"
  };
}

export function started(invocationId, sequence) {
  return envelope(invocationId, sequence, "watch.started", {
    project_root: { encoding: "utf8", value: "/project" }
  });
}

export function cancel(invocationId, sequence) {
  return envelope(invocationId, sequence, "watch.cancel.requested", { cancellation: { type: "user" } });
}

export function stopped(invocationId, sequence) {
  return envelope(invocationId, sequence, "watch.stopped", { reason: { type: "cancelled" } });
}

export function completed(invocationId, sequence) {
  return envelope(invocationId, sequence, "watch.build.completed", {
    generation: 0, snapshot_generation: 0, status: "cancelled", outcome: { type: "cancelled" },
    inputs: [], diagnostics: [], artifacts: [], freshness: { type: "unknown" },
    publication: { type: "not_attempted", reason: "cancelled" }, recovery: [],
    restart_guidance: { type: "host_policy_required", decision: "unspecified" },
    cancellation: { type: "user" }
  });
}

export function envelope(invocationId, sequence, event, data) {
  return { version: 1, sequence, event, command: "watch", invocation_id: invocationId, data };
}

export class FakeChild extends EventEmitter {
  constructor() {
    super();
    this.stdout = new EventEmitter();
    this.stderr = new EventEmitter();
    this.stdin = { writable: true, writes: [], end() {}, destroy() {} };
    this.stdin.write = (value) => { this.stdin.writes.push(JSON.parse(value)); return true; };
    this.killed = false;
  }

  kill(signal) {
    this.killed = true;
    if (signal === "SIGKILL") queueMicrotask(() => this.close(1));
  }
  close(code) { this.emit("close", code, null); }
}
