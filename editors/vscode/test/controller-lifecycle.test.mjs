import test from "node:test";
import assert from "node:assert/strict";
import { ExtensionController } from "../src/controller.js";
import { ClientFailure, ClientFailureKind } from "../src/client-failure.js";
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
  assert.ok(messages.some((message) => message.includes("500 ms")));
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
    .map((message) => message.match(/\d+ ms/)?.[0]), ["100 ms", "100 ms"]);
  await controller.dispose();
});

test("restart exhaustion uses the canonical message without duplicated detail", async () => {
  const messages = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const controller = new ExtensionController(api, output(messages), { delete() {} }, { createClient: () => new FakeClient() });
  await controller.start();
  controller.restartAttempt = 5;
  controller.scheduleRestart();
  assert.equal(messages.at(-1), "Recite language server restart attempts exhausted.");
  await controller.dispose();
});

test("server notifications preserve log visibility and project show-message severity", () => {
  const received = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const userInterface = {
    serverLogMessage: (message) => received.push(["log", message]),
    serverErrorMessage: (message) => received.push(["error", message]),
    serverWarningMessage: (message) => received.push(["warning", message]),
    serverInfoMessage: (message) => received.push(["info", message])
  };
  const controller = new ExtensionController(api, userInterface, { delete() {} });

  controller.handleNotification("window/logMessage", { type: 1, message: "diagnostic detail" });
  controller.handleNotification("window/showMessage", { type: 1, message: "failure" });
  controller.handleNotification("window/showMessage", { type: 2, message: "warning" });
  controller.handleNotification("window/showMessage", { type: 3, message: "information" });

  assert.deepEqual(received, [
    ["log", "diagnostic detail"],
    ["error", "failure"],
    ["warning", "warning"],
    ["info", "information"]
  ]);
});

test("typed client failures select one localized UI category at the controller edge", () => {
  const received = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const userInterface = {
    serverTransportFailure: (detail) => received.push(["transport", detail]),
    serverProtocolFailure: () => received.push(["protocol"]),
    serverLifecycleFailure: (detail) => received.push(["lifecycle", detail])
  };
  const controller = new ExtensionController(api, userInterface, { delete() {} });
  const transportClient = {};
  controller.client = transportClient;
  controller.handleClientFailure(
    transportClient,
    new ClientFailure(ClientFailureKind.Transport, "EPIPE")
  );
  const protocolClient = {};
  controller.client = protocolClient;
  controller.handleClientFailure(
    protocolClient,
    new ClientFailure(ClientFailureKind.Protocol)
  );
  const lifecycleClient = {};
  controller.client = lifecycleClient;
  controller.handleClientFailure(
    lifecycleClient,
    new ClientFailure(ClientFailureKind.Lifecycle, "code=1")
  );

  assert.deepEqual(received, [["transport", "EPIPE"], ["protocol"], ["lifecycle", "code=1"]]);
  assert.equal(transportClient.failureReported, true);
  assert.equal(protocolClient.failureReported, true);
  assert.equal(lifecycleClient.failureReported, true);
});

test("startup failure events are not reported a second time by start rejection", async () => {
  const received = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const userInterface = {
    serverProtocolFailure: () => received.push("protocol")
  };
  const controller = new ExtensionController(api, userInterface, { delete() {} }, {
    createClient: () => {
      const client = new FakeClient();
      client.start = async () => {
        const failure = new ClientFailure(ClientFailureKind.Protocol);
        client.emit("failure", failure);
        throw failure;
      };
      return client;
    }
  });

  assert.equal(await controller.start(), false);
  assert.deepEqual(received, ["protocol"]);
});

test("scheduled retry failures do not repeat visible lifecycle notifications", async () => {
  const received = [];
  let attempts = 0;
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const userInterface = {
    serverLifecycleFailure: (detail) => received.push(["failure", detail]),
    restartScheduled: (detail) => received.push(["scheduled", detail]),
    restartExhausted: () => received.push(["exhausted"]),
    serverLogMessage() {}
  };
  const controller = new ExtensionController(api, userInterface, { delete() {} }, {
    createClient: () => {
      attempts += 1;
      const client = new FakeClient();
      client.start = async () => {
        client.status = "stopped";
        throw new Error("ENOENT");
      };
      return client;
    }
  });

  controller.handleStartFailure(new Error("ENOENT"));
  controller.scheduleRestart();
  await waitFor(() => attempts === 1, 250);

  assert.deepEqual(received.slice(0, 2), [["failure", "ENOENT"], ["scheduled", "100 ms"]]);
  assert.equal(received.filter(([kind]) => kind === "failure").length, 1);
  await controller.dispose();
});
