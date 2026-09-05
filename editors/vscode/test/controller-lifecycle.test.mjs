import test from "node:test";
import assert from "node:assert/strict";
import { ExtensionController } from "../src/controller.js";
import { StartupOutcomeKind } from "../src/startup-outcome.js";
import {
  FakeClient,
  hostApi,
  output,
  uri,
  waitFor
} from "./controller-fixtures.mjs";

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

  assert.equal((await controller.start()).kind, StartupOutcomeKind.Refused);
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

test("controller registers explicit version-safe rename without a native provider", async () => {
  const messages = [];
  const document = {
    languageId: "recite",
    version: 4,
    uri: uri("rename.recite"),
    getText: () => ":: work"
  };
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  api.workspace.textDocuments.push(document);
  api.window.activeTextEditor = {
    document,
    selection: { active: new api.Position(0, 3) }
  };
  api.workspace.applyEdit = async () => true;
  const ui = output(messages, api);
  ui.chooseRenameName = async () => "renamed";
  const requests = [];
  const controller = new ExtensionController(api, ui, { delete() {} }, {
    createClient: () => {
      const client = new FakeClient();
      client.request = async (method) => {
        requests.push(method);
        return method === "textDocument/prepareRename"
          ? { defaultBehavior: true }
          : { documentChanges: [{
            textDocument: { uri: document.uri.toString(), version: document.version },
            edits: [{
              range: { start: { line: 0, character: 3 }, end: { line: 0, character: 7 } },
              newText: "renamed"
            }]
          }] };
      };
      return client;
    }
  });

  await controller.start();
  assert.equal(await api.commands.executeCommand("recite.renameBlock"), true);
  assert.deepEqual(requests, ["textDocument/prepareRename", "textDocument/rename"]);
  assert.equal(api.registeredProviders.some(({ name }) => name === "rename"), false);
  await controller.dispose();
});

test("configuration changes while stopping use the latest settings without a redundant restart", async () => {
  let settings = { path: "initial", args: [], projectRoot: "" };
  let configurationChanged;
  let stoppingEntered;
  let releaseStop;
  const stopGate = new Promise((resolve) => { releaseStop = resolve; });
  const clients = [];
  const starts = [];
  const stops = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  api.workspace.getConfiguration = () => ({
    get: (key, fallback) => ({
      "lsp.path": settings.path,
      "lsp.args": settings.args,
      "lsp.projectRoot": settings.projectRoot
    }[key] ?? fallback)
  });
  api.workspace.onDidChangeConfiguration = (callback) => {
    configurationChanged = callback;
    return { dispose() {} };
  };
  const controller = new ExtensionController(api, output(), { delete() {} }, {
    createClient: (configuration) => {
      const client = new FakeClient();
      clients.push(client);
      const start = client.start.bind(client);
      client.start = async (params) => {
        starts.push({ client, configuration, params });
        await start();
      };
      const stop = client.stop.bind(client);
      client.stop = async () => {
        stops.push(client);
        if (stops.length === 1) {
          stoppingEntered();
          await stopGate;
        }
        await stop();
      };
      return client;
    }
  });
  const stopping = new Promise((resolve) => { stoppingEntered = resolve; });

  await controller.start();
  const restart = controller.restart();
  await stopping;
  settings = { path: "latest", args: ["--latest"], projectRoot: "" };
  configurationChanged({ affectsConfiguration: (section) => section === "recite.lsp" });
  releaseStop();
  await restart;

  assert.equal(starts.length, 2, "a change during stop should not trigger a redundant latest restart");
  assert.deepEqual(starts.map(({ configuration }) => ({
    command: configuration.command,
    args: configuration.args
  })), [
    { command: "initial", args: [] },
    { command: "latest", args: ["--latest"] }
  ]);
  assert.equal(stops.length, 1);
  assert.equal(clients[0].status, "stopped");
  assert.equal(controller.client, clients[1]);
  assert.equal(controller.client.status, "running");
  await controller.dispose();
  assert.equal(stops.length, 2);
});

test("configuration changes during initialization queue one restart with the latest settings", async () => {
  let settings = { path: "initial", args: [], projectRoot: "" };
  let configurationChanged;
  let initializationEntered;
  let releaseInitialization;
  const initializationGate = new Promise((resolve) => { releaseInitialization = resolve; });
  const clients = [];
  const starts = [];
  const stops = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  api.workspace.getConfiguration = () => ({
    get: (key, fallback) => ({
      "lsp.path": settings.path,
      "lsp.args": settings.args,
      "lsp.projectRoot": settings.projectRoot
    }[key] ?? fallback)
  });
  api.workspace.onDidChangeConfiguration = (callback) => {
    configurationChanged = callback;
    return { dispose() {} };
  };
  const controller = new ExtensionController(api, output(), { delete() {} }, {
    createClient: (configuration) => {
      const client = new FakeClient();
      clients.push(client);
      const start = client.start.bind(client);
      client.start = async (params) => {
        starts.push({ client, configuration, params });
        if (starts.length === 2) {
          initializationEntered();
          await initializationGate;
        }
        await start();
      };
      const stop = client.stop.bind(client);
      client.stop = async () => {
        stops.push(client);
        await stop();
      };
      return client;
    }
  });
  const initialization = new Promise((resolve) => { initializationEntered = resolve; });

  await controller.start();
  settings = { path: "first-restart", args: ["--first"], projectRoot: "" };
  const restart = controller.restart();
  await initialization;

  settings = { path: "latest", args: ["--latest"], projectRoot: "" };
  configurationChanged({ affectsConfiguration: (section) => section === "recite.lsp" });
  releaseInitialization();
  await restart;

  assert.equal(starts.length, 3, "the queued change should cause exactly one follow-up start");
  assert.deepEqual(starts.map(({ configuration }) => ({
    command: configuration.command,
    args: configuration.args
  })), [
    { command: "initial", args: [] },
    { command: "first-restart", args: ["--first"] },
    { command: "latest", args: ["--latest"] }
  ]);
  assert.equal(stops.length, 2);
  assert.equal(clients[0].status, "stopped");
  assert.equal(clients[1].status, "stopped");
  assert.equal(controller.client, clients[2]);
  assert.equal(controller.client.status, "running");
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
  const controller = new ExtensionController(api, output(messages), { delete() {} }, {
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
  assert.ok(messages.some((message) => message.includes("500 milliseconds")));
  await controller.dispose();
});

test("restart budget resets only after a separated stable run", async () => {
  const clients = [];
  const messages = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const controller = new ExtensionController(api, output(messages), { delete() {} }, {
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
    .map((message) => message.match(/\d+ milliseconds/)?.[0]),
  ["100 milliseconds", "100 milliseconds"]);
  await controller.dispose();
});

test("restart exhaustion uses the canonical message without duplicated detail", async () => {
  const messages = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const controller = new ExtensionController(api, output(messages), { delete() {} }, {
    createClient: () => new FakeClient(), restartDelaysMs: []
  });
  await controller.start();
  controller.scheduleRestart();
  assert.equal(messages.at(-1), "Recite language server restart attempts exhausted.");
  await controller.dispose();
});
