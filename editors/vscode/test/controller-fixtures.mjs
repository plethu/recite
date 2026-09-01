import { EventEmitter } from "node:events";

export function uri(value) {
  return { toString: () => `file:///${value}` };
}

export function action(document, title) {
  return {
    title,
    kind: "quickfix",
    edit: { documentChanges: [{
      textDocument: { uri: document.uri.toString(), version: document.version },
      edits: [{
        range: { start: { line: 0, character: 0 }, end: { line: 0, character: 2 } },
        newText: "# "
      }]
    }] }
  };
}

export function output(messages = []) {
  return {
    append() {},
    appendLine(value) { messages.push(value); },
    serverTransportFailure(detail) { this.appendLine(`Recite language server transport failed: ${detail}.`); },
    serverProtocolFailure() { this.appendLine("Recite language server protocol failed."); },
    serverLifecycleFailure(detail) { this.appendLine(`Recite language server lifecycle failed: ${detail}.`); },
    serverExited() { this.appendLine("Recite language server exited."); },
    restartScheduled(milliseconds) {
      this.appendLine(`Recite language server restart scheduled in ${milliseconds} milliseconds.`);
    },
    restartExhausted() { this.appendLine("Recite language server restart attempts exhausted."); },
    actionStale() { this.appendLine("Recite code action is no longer applicable because the document changed."); },
    actionClosed() { this.appendLine("Recite code action is no longer applicable because the document closed."); },
    actionReopened() {
      this.appendLine("Recite code action is no longer applicable because the document was closed and reopened.");
    },
    actionExpired() { this.appendLine("Recite code action expired before it was applied."); },
    actionEvicted() { this.appendLine("Recite code action was replaced by a newer action."); },
    actionApplyFailed() { this.appendLine("VS Code could not apply the Recite code action."); },
    actionUnknown() { this.appendLine("Recite code action is no longer available."); },
    configurationPathInvalid: () => new Error("recite.lsp.path must be a non-empty string."),
    configurationArgsInvalid: () => new Error("recite.lsp.args must be an array of strings."),
    configurationProjectRootInvalid: () => new Error("recite.lsp.projectRoot must be a string."),
    configurationProjectRootNeedsWorkspace: () =>
      new Error("recite.lsp.projectRoot needs a workspace for relative paths."),
    serverNotRunning: () => new Error("Recite language server is not running."),
    serverStderr() {},
    serverLogMessage() {},
    serverErrorMessage() {},
    serverWarningMessage() {},
    serverInfoMessage() {},
    dispose() {}
  };
}

export class FakeClient extends EventEmitter {
  status = "idle";
  notifications = [];
  async start() { this.status = "running"; }
  async stop() { this.status = "stopped"; }
  notify(method, params) { this.notifications.push({ method, params }); return true; }
}

export function hostApi({ isTrusted, onDidGrantWorkspaceTrust }) {
  const registeredProviders = [];
  const registeredCommands = new Map();
  const workspace = {
    get isTrusted() { return isTrusted(); },
    onDidGrantWorkspaceTrust,
    workspaceFolders: [],
    textDocuments: [],
    getConfiguration: () => ({ get: (_key, fallback) => fallback }),
    applyEdit: async () => true
  };
  for (const event of [
    "onDidOpenTextDocument", "onDidChangeTextDocument", "onDidSaveTextDocument",
    "onDidCloseTextDocument", "onDidChangeConfiguration"
  ]) workspace[event] = () => ({ dispose() {} });
  const api = {
    workspace,
    languages: {
      registerCompletionItemProvider: register("completion"),
      registerHoverProvider: register("hover"),
      registerDefinitionProvider: register("definition"),
      registerReferenceProvider: register("reference"),
      registerRenameProvider: register("rename"),
      registerCodeActionsProvider: register("code-actions")
    },
    commands: {
      registerCommand: (name, callback) => {
        registeredCommands.set(name, callback);
        return { dispose: () => registeredCommands.delete(name) };
      },
      executeCommand: (name, ...args) => registeredCommands.get(name)?.(...args)
    },
    registeredProviders,
    Uri: {
      file: (value) => ({ toString: () => `file://${value}` }),
      parse: (value) => ({ toString: () => value })
    },
    Command: class Command {
      constructor(title, command, ...args) { this.title = title; this.command = command; this.arguments = args; }
    },
    Position: class Position {
      constructor(line, character) { this.line = line; this.character = character; }
    },
    Range: class Range {
      constructor(start, end) { this.start = start; this.end = end; }
    },
    WorkspaceEdit: class WorkspaceEdit {
      constructor() { this.replacements = []; }
      replace(uri, range, newText) { this.replacements.push({ uri, range, newText }); }
    },
    CodeAction: class CodeAction {
      constructor(title, kind) { this.title = title; this.kind = kind; }
    },
    CodeActionKind: { QuickFix: "quickfix", Refactor: "refactor", Source: "source", Empty: "" }
  };
  return api;

  function register(name) {
    return (selector, provider, ...triggerCharacters) => {
      registeredProviders.push({ name, selector, provider, triggerCharacters });
      return { dispose() {} };
    };
  }
}

export async function waitFor(predicate, timeoutMs = 5_000) {
  const deadline = Date.now() + timeoutMs;
  while (!predicate()) {
    if (Date.now() >= deadline) throw new Error("timed out waiting for controller evidence");
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
}
