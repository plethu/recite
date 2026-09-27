import test from "node:test";
import assert from "node:assert/strict";
import { ReciteStandardClient } from "../src/standard-client.js";
import { hostApi, action } from "./controller-fixtures.mjs";

class FakeLanguageClient {
  constructor(_id, _name, server, options) {
    this.server = server;
    this.options = options;
    this.requests = [];
    FakeLanguageClient.current = this;
  }
  onDidChangeState(listener) { this.stateListener = listener; return { dispose() {} }; }
  async start() { this.started = true; }
  async stop() { this.stopped = true; }
  async sendRequest(method, params, token) {
    this.requestArgumentCount = arguments.length;
    this.requests.push({ method, params, token });
    return this.response;
  }
}
const load = async () => ({
  LanguageClient: FakeLanguageClient,
  State: { Stopped: 3 },
  ErrorAction: { Shutdown: 2 },
  CloseAction: { DoNotRestart: 2 }
});

test("standard client delegates synchronization and language features to the library", async () => {
  const api = hostApi({ isTrusted: () => true });
  const controller = {
    createEditCommandBatch: () => ({ finish() {} }),
    createEditCommand: () => ({ title: "Apply", command: "recite.applyCodeAction" })
  };
  const client = new ReciteStandardClient(api,
    { command: "recite-lsp", args: ["--stdio"], cwd: "/project", projectRootOverridden: true }, controller, load);
  await client.start();
  const implementation = FakeLanguageClient.current;
  assert.equal(client.status, "running");
  assert.deepEqual(implementation.server, {
    command: "recite-lsp", args: ["--stdio"], options: { cwd: "/project", shell: false }
  });
  assert.deepEqual(implementation.options.documentSelector.map((entry) => entry.scheme), ["file", "untitled"]);
  assert.equal(implementation.options.workspaceFolder.uri.toString(), "file:///project");
  assert.equal(api.registeredProviders.length, 0);
  assert.equal(implementation.options.middleware.provideRenameEdits(), undefined);
  await client.request("textDocument/prepareRename", { textDocument: { uri: "file:///project/start.recite" } });
  assert.equal(implementation.requestArgumentCount, 2,
    "raw requests without cancellation must not pass an undefined token");
  await client.stop();
  assert.equal(implementation.stopped, true);
});

test("multi-root workspaces leave folder discovery to the language client", async () => {
  const api = hostApi({ isTrusted: () => true });
  api.workspace.workspaceFolders = ["/first", "/second"].map((root, index) => ({
    name: root.slice(1), uri: api.Uri.file(root), index
  }));
  const client = new ReciteStandardClient(api,
    { command: "recite-lsp", args: [], cwd: "/first", projectRootOverridden: false },
    {}, load);

  await client.start();
  assert.equal(FakeLanguageClient.current.options.workspaceFolder, undefined);
  assert.equal(FakeLanguageClient.current.server.options.cwd, "/first");
  await client.stop();
});

test("code actions keep version guards and cancellation at the Recite boundary", async () => {
  const api = hostApi({ isTrusted: () => true });
  const document = { uri: api.Uri.file("/project/start.recite"), version: 2, languageId: "recite" };
  api.workspace.textDocuments.push(document);
  const edits = [];
  const controller = {
    createEditCommandBatch: () => ({ finish() {} }),
    createEditCommand: (_title, edit) => { edits.push(edit); return { command: "recite.applyCodeAction" }; }
  };
  const client = new ReciteStandardClient(api,
    { command: "recite-lsp", args: [], cwd: "/project" }, controller, load);
  await client.start();
  const implementation = FakeLanguageClient.current;
  implementation.response = [action(document, "Add ID")];
  const result = await implementation.options.middleware.provideCodeActions(document,
    { start: { line: 0, character: 0 }, end: { line: 0, character: 2 } },
    { diagnostics: [] }, { isCancellationRequested: false });
  assert.equal(result[0].command.command, "recite.applyCodeAction");
  assert.equal(edits[0].reciteVersionStatus(), "current");
  document.version += 1;
  assert.equal(edits[0].reciteVersionStatus(), "document-stale");
  assert.equal(implementation.requests[0].method, "textDocument/codeAction");
  await client.stop();
});
