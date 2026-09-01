import test from "node:test";
import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { ExtensionController } from "../src/controller.js";
import { lspWorkspaceEditToVscode } from "../src/lsp-features.js";

test("dynamic watched-file registration forwards deterministic events and disposes", async () => {
  const callbacks = {};
  const disposables = [];
  const notifications = [];
  const api = {
    workspace: {
      createFileSystemWatcher: () => ({
        onDidCreate: (callback) => hook("create", callback),
        onDidChange: (callback) => hook("change", callback),
        onDidDelete: (callback) => hook("delete", callback),
        dispose: () => disposables.push("watcher")
      })
    }
  };
  const controller = new ExtensionController(api, output(), { delete() {} });
  controller.client = { status: "running", notify: (method, params) => notifications.push({ method, params }) };

  await controller.registerCapabilities({ registrations: [{
    id: "files",
    method: "workspace/didChangeWatchedFiles",
    registerOptions: { watchers: [{ globPattern: "**/*", kind: 7 }] }
  }] });
  callbacks.create(uri("z.recite"));
  callbacks.delete(uri("a.recite"));
  callbacks.change(uri("m.recite"));
  await new Promise((resolve) => setTimeout(resolve, 5));

  assert.deepEqual(notifications, [{
    method: "workspace/didChangeWatchedFiles",
    params: { changes: [
      { type: 1, uri: "file:///z.recite" },
      { type: 2, uri: "file:///m.recite" },
      { type: 3, uri: "file:///a.recite" }
    ] }
  }]);
  controller.unregisterCapabilities({ unregisterations: [{ id: "files" }] });
  assert.deepEqual(disposables, ["watcher"]);

  function hook(kind, callback) {
    callbacks[kind] = callback;
    return { dispose() {} };
  }
});

test("untrusted workspaces never spawn and start once trust is granted", async () => {
  let trusted = false;
  let grantTrust;
  let clientCreated = 0;
  const api = hostApi({
    isTrusted: () => trusted,
    onDidGrantWorkspaceTrust: (callback) => { grantTrust = callback; return { dispose() {} }; }
  });
  const controller = new ExtensionController(api, output(), { delete() {} }, {
    createClient: () => { clientCreated += 1; return new FakeClient(); }
  });

  assert.equal(await controller.start(), false);
  assert.equal(clientCreated, 0);
  trusted = true;
  await grantTrust();
  assert.equal(clientCreated, 1);
});

test("activation-shaped startup replays documents already open before activation", async () => {
  const document = {
    languageId: "recite",
    version: 7,
    uri: uri("already-open.recite"),
    getText: () => ":: already-open"
  };
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  api.workspace.textDocuments.push(document);
  const clients = [];
  const controller = new ExtensionController(api, output(), { delete() {} }, {
    createClient: () => {
      const client = new FakeClient();
      clients.push(client);
      return client;
    }
  });

  await controller.start();

  assert.deepEqual(clients[0].notifications, [{
    method: "textDocument/didOpen",
    params: { textDocument: {
      uri: "file:///already-open.recite", languageId: "recite", version: 7,
      text: ":: already-open"
    } }
  }]);
  assert.equal(api.registeredProviders.some(({ name }) => name === "rename"), false);
  await controller.dispose();
});

test("a crashed server restarts with bounded backoff and replays open documents", async () => {
  const clients = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const controller = new ExtensionController(api, output(), { delete() {} }, {
    createClient: () => {
      const client = new FakeClient();
      clients.push(client);
      return client;
    }
  });
  const document = {
    languageId: "recite",
    version: 3,
    uri: uri("dialogue.recite"),
    getText: () => ":: intro"
  };

  await controller.start();
  controller.open(document);
  clients[0].emit("exit", { code: 1, signal: null });
  await new Promise((resolve) => setTimeout(resolve, 125));

  assert.equal(clients.length, 2);
  assert.deepEqual(clients[1].notifications, [{
    method: "textDocument/didOpen",
    params: { textDocument: {
      uri: "file:///dialogue.recite", languageId: "recite", version: 3, text: ":: intro"
    } }
  }]);
  await controller.dispose();
  assert.equal(clients[1].status, "stopped");
});

test("rapid crash loops retain backoff until a stable run completes", async () => {
  const clients = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const messages = [];
  const controller = new ExtensionController(api, {
    append() {},
    appendLine(value) { messages.push(value); }
  }, { delete() {} }, {
    stableRunMs: 1_000,
    createClient: () => {
      const client = new FakeClient();
      clients.push(client);
      return client;
    }
  });

  await controller.start();
  clients[0].emit("exit", { code: 1, signal: null });
  await waitFor(() => clients.length === 2, 250);
  clients[1].emit("exit", { code: 1, signal: null });
  await waitFor(() => clients.length === 3, 750);

  assert.equal(messages.filter((message) => message.includes("restart scheduled")).length, 2);
  assert.ok(messages.some((message) => message.includes("500 ms")));
  await controller.dispose();
});

test("restart budget resets only after a separated stable run", async () => {
  const clients = [];
  const messages = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const controller = new ExtensionController(api, {
    append() {},
    appendLine(value) { messages.push(value); }
  }, { delete() {} }, {
    stableRunMs: 25,
    createClient: () => {
      const client = new FakeClient();
      clients.push(client);
      return client;
    }
  });

  await controller.start();
  clients[0].emit("exit", { code: 1, signal: null });
  await waitFor(() => clients.length === 2, 250);
  await new Promise((resolve) => setTimeout(resolve, 40));
  clients[1].emit("exit", { code: 1, signal: null });
  await waitFor(() => clients.length === 3, 250);

  assert.deepEqual(messages.filter((message) => message.includes("restart scheduled"))
    .map((message) => message.match(/\d+ ms/)?.[0]), ["100 ms", "100 ms"]);
  await controller.dispose();
});

test("restart exhaustion uses the canonical message without duplicated detail", async () => {
  const messages = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const controller = new ExtensionController(api, {
    append() {},
    appendLine(value) { messages.push(value); }
  }, { delete() {} }, { createClient: () => new FakeClient() });
  await controller.start();
  controller.restartAttempt = 5;
  controller.scheduleRestart();
  assert.equal(messages.at(-1), "Recite language server restart attempts exhausted.");
  await controller.dispose();
});

test("controller-owned edit commands revalidate immediately before apply", async () => {
  const applied = [];
  const messages = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  api.workspace.applyEdit = async (edit) => {
    applied.push(edit);
    return true;
  };
  const controller = new ExtensionController(api, {
    append() {},
    appendLine(value) { messages.push(value); }
  }, { delete() {} }, {
    createClient: () => new FakeClient()
  });
  await controller.start();

  let current = true;
  const edit = { reciteVersionGuard: () => current };
  const command = controller.createEditCommand("Apply fix", edit);
  assert.equal(await api.commands.executeCommand(command.command, ...command.arguments), true);
  assert.deepEqual(applied, [edit]);

  const stale = { reciteVersionGuard: () => false };
  const staleCommand = controller.createEditCommand("Stale fix", stale);
  current = false;
  assert.equal(await api.commands.executeCommand(staleCommand.command, ...staleCommand.arguments), false);
  assert.deepEqual(applied, [edit]);
  assert.equal(messages.at(-1), "Recite code action is no longer applicable because the document changed.");
  await controller.dispose();
});

test("code-action command cache reports capacity eviction and TTL expiry", async () => {
  const messages = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const controller = new ExtensionController(api, {
    append() {},
    appendLine(value) { messages.push(value); }
  }, { delete() {} }, {
    editCommandTtlMs: 15,
    maxEditCommands: 1,
    createClient: () => new FakeClient()
  });
  await controller.start();

  const first = controller.createEditCommand("First", { reciteVersionGuard: () => true });
  const second = controller.createEditCommand("Second", { reciteVersionGuard: () => true });
  assert.equal(await api.commands.executeCommand(first.command, ...first.arguments), false);
  assert.equal(messages.at(-1), "Recite code action was replaced by a newer action.");
  await new Promise((resolve) => setTimeout(resolve, 25));
  assert.equal(await api.commands.executeCommand(second.command, ...second.arguments), false);
  assert.equal(messages.at(-1), "Recite code action expired before it was applied.");
  await controller.dispose();
});

test("unknown code-action IDs and host apply failures have distinct outcomes", async () => {
  const messages = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  api.workspace.applyEdit = async () => false;
  const controller = new ExtensionController(api, {
    append() {},
    appendLine(value) { messages.push(value); }
  }, { delete() {} }, { createClient: () => new FakeClient() });
  await controller.start();

  assert.equal(await api.commands.executeCommand("recite.applyCodeAction", "missing"), false);
  assert.equal(messages.at(-1), "Recite code action is no longer available.");
  const command = controller.createEditCommand("Rejected", { reciteVersionGuard: () => true });
  assert.equal(await api.commands.executeCommand(command.command, ...command.arguments), false);
  assert.equal(messages.at(-1), "VS Code could not apply the Recite code action.");
  await controller.dispose();
});

test("a capacity-one projected response never returns an evicted command", async () => {
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  api.workspace.applyEdit = async () => true;
  const primary = {
    languageId: "recite",
    version: 4,
    uri: uri("dialogue.recite"),
    getText: () => ":: dialogue"
  };
  api.workspace.textDocuments.push(primary);
  const controller = new ExtensionController(api, output(), { delete() {} }, {
    maxEditCommands: 1,
    createClient: () => {
      const client = new FakeClient();
      client.request = async () => [
        action(primary, "First"),
        action(primary, "Second")
      ];
      return client;
    }
  });
  await controller.start();
  const provider = api.registeredProviders.find(({ name }) => name === "code-actions").provider;
  const actions = await provider.provideCodeActions(
    primary,
    new api.Range(new api.Position(0, 0), new api.Position(0, 2)),
    { diagnostics: [] }
  );
  assert.equal(actions.length, 1);
  assert.equal(await api.commands.executeCommand(
    actions[0].command.command, ...actions[0].command.arguments
  ), true);
  await controller.dispose();
});

test("document close reports a closed command instead of an unknown ID", async () => {
  const messages = [];
  const document = {
    languageId: "recite",
    version: 4,
    uri: uri("dialogue.recite"),
    getText: () => ":: dialogue"
  };
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  api.workspace.textDocuments.push(document);
  const controller = new ExtensionController(api, {
    append() {},
    appendLine(value) { messages.push(value); }
  }, { delete() {} }, { createClient: () => new FakeClient() });
  await controller.start();
  const edit = {
    reciteVersionPreconditions: [{ document }],
    reciteVersionGuard: () => false
  };
  const command = controller.createEditCommand("Close", edit);
  controller.discardEditCommandsForDocument(document, "document-closed");
  assert.equal(await api.commands.executeCommand(command.command, ...command.arguments), false);
  assert.equal(messages.at(-1), "Recite code action is no longer applicable because the document closed.");
  await controller.dispose();
});

test("document reopen reports a new generation instead of applying an old action", async () => {
  const messages = [];
  const document = {
    languageId: "recite",
    version: 4,
    uri: uri("dialogue.recite"),
    getText: () => ":: dialogue"
  };
  const reopened = { ...document };
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  api.workspace.textDocuments.push(document);
  const controller = new ExtensionController(api, {
    append() {},
    appendLine(value) { messages.push(value); }
  }, { delete() {} }, { createClient: () => new FakeClient() });
  await controller.start();
  const edit = lspWorkspaceEditToVscode(api, {
    documentChanges: [{
      textDocument: { uri: document.uri.toString(), version: document.version },
      edits: []
    }]
  }, (documentUri) => api.workspace.textDocuments.find(
    (candidate) => candidate.uri.toString() === documentUri.toString()
  ));
  const command = controller.createEditCommand("Reopen", edit);
  api.workspace.textDocuments[0] = reopened;
  assert.equal(await api.commands.executeCommand(command.command, ...command.arguments), false);
  assert.equal(messages.at(-1), "Recite code action is no longer applicable because the document was closed and reopened.");
  await controller.dispose();
});

test("activation-shaped host API routes versioned code actions through the command boundary", async () => {
  const applied = [];
  const primary = {
    languageId: "recite",
    version: 4,
    uri: uri("dialogue.recite"),
    getText: () => ":: dialogue"
  };
  const sibling = {
    languageId: "recite",
    version: 9,
    uri: uri("sibling.recite"),
    getText: () => ":: sibling"
  };
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  api.workspace.textDocuments.push(primary, sibling);
  api.workspace.applyEdit = async (edit) => {
    applied.push(edit);
    return true;
  };
  const controller = new ExtensionController(api, output(), { delete() {} }, {
    createClient: () => {
      const client = new FakeClient();
      client.request = async () => [{
        kind: "quickfix",
        title: "Apply fix",
        edit: { documentChanges: [
          {
            textDocument: { uri: primary.uri.toString(), version: 4 },
            edits: [{
              range: { start: { line: 0, character: 0 }, end: { line: 0, character: 2 } },
              newText: "# "
            }]
          },
          { textDocument: { uri: sibling.uri.toString(), version: 9 }, edits: [] }
        ] }
      }];
      return client;
    }
  });
  await controller.start();
  const provider = api.registeredProviders.find(({ name }) => name === "code-actions").provider;
  const actions = await provider.provideCodeActions(
    primary,
    new api.Range(new api.Position(0, 0), new api.Position(0, 2)),
    { diagnostics: [] }
  );
  assert.equal(actions.length, 1);
  assert.equal(actions[0].edit, undefined);
  assert.equal(actions[0].command.command, "recite.applyCodeAction");

  sibling.version = 10;
  assert.equal(await api.commands.executeCommand(
    actions[0].command.command, ...actions[0].command.arguments
  ), false);
  assert.deepEqual(applied, []);
  await controller.dispose();
});

function uri(value) {
  return { toString: () => `file:///${value}` };
}

function action(document, title) {
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

function output() {
  return { append() {}, appendLine() {} };
}

class FakeClient extends EventEmitter {
  status = "idle";
  notifications = [];
  async start() { this.status = "running"; }
  async stop() { this.status = "stopped"; }
  notify(method, params) { this.notifications.push({ method, params }); return true; }
}

function hostApi({ isTrusted, onDidGrantWorkspaceTrust }) {
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

async function waitFor(predicate, timeoutMs = 5_000) {
  const deadline = Date.now() + timeoutMs;
  while (!predicate()) {
    if (Date.now() >= deadline) throw new Error("timed out waiting for controller evidence");
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
}
