import test from "node:test";
import assert from "node:assert/strict";
import { ExtensionController } from "../src/controller.js";
import { StartupOutcomeKind } from "../src/startup-outcome.js";
import { FakeClient, hostApi, output, uri, waitFor } from "./controller-fixtures.mjs";

function diagnostic(message) {
  return {
    range: { start: { line: 0, character: 0 }, end: { line: 0, character: 1 } },
    message,
    severity: 1
  };
}

test("retiring a server generation clears diagnostics and rejects late publishes", async () => {
  const document = {
    languageId: "recite",
    version: 4,
    uri: uri("dialogue.recite"),
    getText: () => ":: dialogue"
  };
  const entries = new Map();
  let clears = 0;
  const diagnostics = {
    set: (diagnosticUri, values) => entries.set(diagnosticUri.toString(), values),
    clear: () => { clears += 1; entries.clear(); },
    delete: (diagnosticUri) => entries.delete(diagnosticUri.toString())
  };
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  let projectRoot = "/old-root";
  api.workspace.getConfiguration = () => ({
    get: (key, fallback) => key === "lsp.projectRoot" ? projectRoot : fallback
  });
  api.workspace.textDocuments.push(document);
  const clients = [];
  const starts = [];
  const controller = new ExtensionController(api, output(), diagnostics, {
    createClient: () => {
      const client = new FakeClient();
      clients.push(client);
      const start = client.start.bind(client);
      client.start = async (params) => {
        starts.push(params);
        await start(params);
      };
      return client;
    }
  });

  await controller.start();
  assert.equal(starts[0].rootUri, "file:///old-root");
  clients[0].emit("notification", "textDocument/publishDiagnostics", {
    uri: document.uri.toString(), diagnostics: [diagnostic("old generation")], version: 4
  });
  assert.equal(entries.size, 1);

  projectRoot = "/new-root";
  await controller.restart();
  assert.equal(starts[1].rootUri, "file:///new-root");
  assert.equal(clears, 1);
  assert.equal(entries.size, 0);

  clients[0].emit("notification", "textDocument/publishDiagnostics", {
    uri: document.uri.toString(), diagnostics: [diagnostic("late old generation")], version: 4
  });
  assert.equal(entries.size, 0, "a retired client must not repopulate diagnostics");

  clients[1].emit("notification", "textDocument/publishDiagnostics", {
    uri: document.uri.toString(), diagnostics: [diagnostic("new generation")], version: 4
  });
  assert.deepEqual(entries.get(document.uri.toString()).map(({ message }) => message), ["new generation"]);

  api.workspace.textDocuments.length = 0;
  clients[0].emit("notification", "textDocument/publishDiagnostics", {
    uri: document.uri.toString(), diagnostics: [diagnostic("closed old generation")], version: 4
  });
  assert.equal(entries.size, 1, "closed-document publishes from a retired client are ignored");
  api.workspace.textDocuments.push({ ...document });
  clients[0].emit("notification", "textDocument/publishDiagnostics", {
    uri: document.uri.toString(), diagnostics: [diagnostic("reopened old generation")], version: 4
  });
  assert.deepEqual(entries.get(document.uri.toString()).map(({ message }) => message), ["new generation"]);
  await controller.dispose();
});

test("an in-place start leaves the current generation diagnostics intact", async () => {
  const document = {
    languageId: "recite",
    version: 1,
    uri: uri("stable.recite"),
    getText: () => ":: stable"
  };
  const entries = new Map();
  let clears = 0;
  const diagnostics = {
    set: (diagnosticUri, values) => entries.set(diagnosticUri.toString(), values),
    clear: () => { clears += 1; entries.clear(); },
    delete() {}
  };
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  api.workspace.textDocuments.push(document);
  const client = new FakeClient();
  const controller = new ExtensionController(api, output(), diagnostics, {
    createClient: () => client
  });

  await controller.start();
  client.emit("notification", "textDocument/publishDiagnostics", {
    uri: document.uri.toString(), diagnostics: [diagnostic("stable")], version: 1
  });
  await controller.start();

  assert.equal(clears, 0);
  assert.equal(entries.size, 1);
  await controller.dispose();
});

test("workspace folder changes queue one latest-authority restart", async () => {
  let folderChanged;
  let configurationChanged;
  let settings = { projectRoot: "" };
  let releaseStop;
  let stoppingEntered;
  const stopGate = new Promise((resolve) => { releaseStop = resolve; });
  const stopped = new Promise((resolve) => { stoppingEntered = resolve; });
  const first = { name: "first", uri: { fsPath: "/workspace/first", toString: () => "file:///workspace/first" } };
  const second = { name: "second", uri: { fsPath: "/workspace/second", toString: () => "file:///workspace/second" } };
  const third = { name: "third", uri: { fsPath: "/workspace/third", toString: () => "file:///workspace/third" } };
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  api.workspace.workspaceFolders = [first];
  api.workspace.getConfiguration = () => ({
    get: (key, fallback) => ({ "lsp.projectRoot": settings.projectRoot }[key] ?? fallback)
  });
  api.workspace.onDidChangeWorkspaceFolders = (callback) => {
    folderChanged = callback;
    return { dispose() {} };
  };
  api.workspace.onDidChangeConfiguration = (callback) => {
    configurationChanged = callback;
    return { dispose() {} };
  };
  const clients = [];
  const starts = [];
  const controller = new ExtensionController(api, output(), { delete() {} }, {
    createClient: () => {
      const client = new FakeClient();
      clients.push(client);
      const start = client.start.bind(client);
      client.start = async (params) => {
        starts.push(params);
        await start(params);
      };
      const stop = client.stop.bind(client);
      client.stop = async () => {
        if (clients.length === 1) {
          stoppingEntered();
          await stopGate;
        }
        await stop();
      };
      return client;
    }
  });

  await controller.start();
  assert.deepEqual(starts[0].workspaceFolders, [{ name: "first", uri: "file:///workspace/first" }]);

  api.workspace.workspaceFolders = [first, second];
  folderChanged({ added: [second], removed: [] });
  await stopped;
  api.workspace.workspaceFolders = [first, second, third];
  folderChanged({ added: [third], removed: [] });
  api.workspace.workspaceFolders = [first, third];
  folderChanged({ added: [], removed: [second] });
  configurationChanged({ affectsConfiguration: (section) => section === "recite.lsp" });
  releaseStop();
  await waitFor(() => starts.length === 2 && controller.restartPromise === undefined);

  assert.deepEqual(starts[1].workspaceFolders, [
    { name: "first", uri: "file:///workspace/first" },
    { name: "third", uri: "file:///workspace/third" }
  ]);

  settings.projectRoot = "/configured-root";
  api.workspace.workspaceFolders = [first, second];
  folderChanged({ added: [second], removed: [third] });
  await waitFor(() => starts.length === 3 && controller.restartPromise === undefined);
  assert.equal(starts[2].rootUri, "file:///configured-root");
  assert.deepEqual(starts[2].workspaceFolders, [{
    name: "configured-root", uri: "file:///configured-root"
  }]);
  await controller.dispose();
});

test("an explicit authority restart cancels a pending crash retry", async () => {
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const clients = [];
  const controller = new ExtensionController(api, output(), { delete() {} }, {
    restartDelaysMs: [100],
    createClient: () => {
      const client = new FakeClient();
      clients.push(client);
      return client;
    }
  });

  await controller.start();
  clients[0].emit("exit", { code: 1, signal: null });
  assert.notEqual(controller.restartTimer, undefined);
  await controller.restart();
  await new Promise((resolve) => setTimeout(resolve, 125));

  assert.equal(clients.length, 2);
  await controller.dispose();
});

test("dispose retires the generation before awaiting graceful client stop", async () => {
  const document = {
    languageId: "recite",
    version: 1,
    uri: uri("disposing.recite"),
    getText: () => ":: disposing"
  };
  const entries = new Map();
  let clears = 0;
  const diagnostics = {
    set: (diagnosticUri, values) => entries.set(diagnosticUri.toString(), values),
    clear: () => { clears += 1; entries.clear(); },
    delete() {}
  };
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  api.workspace.textDocuments.push(document);
  let releaseStop;
  const stopGate = new Promise((resolve) => { releaseStop = resolve; });
  let stopEntered;
  const stopped = new Promise((resolve) => { stopEntered = resolve; });
  let client;
  const controller = new ExtensionController(api, output(), diagnostics, {
    createClient: () => {
      client = new FakeClient();
      const stop = client.stop.bind(client);
      client.stop = async () => {
        stopEntered();
        await stopGate;
        await stop();
      };
      return client;
    }
  });

  await controller.start();
  client.emit("notification", "textDocument/publishDiagnostics", {
    uri: document.uri.toString(), diagnostics: [diagnostic("before dispose")], version: 1
  });
  assert.equal(entries.size, 1);

  const disposal = controller.dispose();
  await stopped;
  assert.equal(clears, 1);
  assert.equal(entries.size, 0);
  client.emit("notification", "textDocument/publishDiagnostics", {
    uri: document.uri.toString(), diagnostics: [diagnostic("during dispose")], version: 1
  });
  assert.equal(entries.size, 0, "a stopping generation must not repopulate diagnostics");
  releaseStop();
  await disposal;
  assert.equal(client.status, "stopped");
});

test("dispose during suspended start prevents the client from reviving", async () => {
  let releaseStart;
  const startGate = new Promise((resolve) => { releaseStart = resolve; });
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  let client;
  const controller = new ExtensionController(api, output(), { clear() {} }, {
    createClient: () => {
      client = new FakeClient();
      client.start = async () => {
        await startGate;
        client.status = "running";
      };
      return client;
    }
  });

  const starting = controller.start();
  await waitFor(() => client !== undefined);
  await controller.dispose();
  releaseStart();
  const outcome = await starting;

  assert.equal(outcome.kind, StartupOutcomeKind.Refused);
  assert.equal(client.status, "stopped");
  assert.equal(controller.client, undefined);
  assert.equal(controller.stableRunTimer, undefined);
  assert.equal(controller.restartTimer, undefined);
});
