import test from "node:test";
import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { ExtensionController } from "../src/controller.js";

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

function uri(value) {
  return { toString: () => `file:///${value}` };
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
  const workspace = {
    get isTrusted() { return isTrusted(); },
    onDidGrantWorkspaceTrust,
    workspaceFolders: [],
    textDocuments: [],
    getConfiguration: () => ({ get: (_key, fallback) => fallback })
  };
  for (const event of [
    "onDidOpenTextDocument", "onDidChangeTextDocument", "onDidSaveTextDocument",
    "onDidCloseTextDocument", "onDidChangeConfiguration"
  ]) workspace[event] = () => ({ dispose() {} });
  return {
    workspace,
    languages: {
      registerCompletionItemProvider: () => ({ dispose() {} }),
      registerHoverProvider: () => ({ dispose() {} }),
      registerDefinitionProvider: () => ({ dispose() {} }),
      registerReferenceProvider: () => ({ dispose() {} }),
      registerRenameProvider: () => ({ dispose() {} }),
      registerCodeActionsProvider: () => ({ dispose() {} })
    },
    Uri: {
      file: (value) => ({ toString: () => `file://${value}` }),
      parse: (value) => ({ toString: () => value })
    }
  };
}
