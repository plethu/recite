import test from "node:test";
import assert from "node:assert/strict";
import { ExtensionController } from "../src/controller.js";
import { lspWorkspaceEditToVscode } from "../src/lsp-features.js";
import { FakeClient, hostApi, output, uri } from "./controller-fixtures.mjs";

test("controller-owned edit commands revalidate immediately before apply", async () => {
  const applied = [];
  const messages = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  api.workspace.applyEdit = async (edit) => {
    applied.push(edit);
    return true;
  };
  const controller = new ExtensionController(api, output(messages), { delete() {} }, {
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
  const controller = new ExtensionController(api, output(messages), { delete() {} }, {
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
  const controller = new ExtensionController(api, output(messages), { delete() {} }, { createClient: () => new FakeClient() });
  await controller.start();

  assert.equal(await api.commands.executeCommand("recite.applyCodeAction", "missing"), false);
  assert.equal(messages.at(-1), "Recite code action is no longer available.");
  const command = controller.createEditCommand("Rejected", { reciteVersionGuard: () => true });
  assert.equal(await api.commands.executeCommand(command.command, ...command.arguments), false);
  assert.equal(messages.at(-1), "VS Code could not apply the Recite code action.");
  await controller.dispose();
});

test("retired command outcomes stay bounded and expire to unknown", async () => {
  const messages = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const controller = new ExtensionController(api, output(messages), { delete() {} }, {
    maxEditCommands: 1,
    maxRetiredCommands: 1,
    editCommandTtlMs: 1_000,
    retiredCommandTtlMs: 15,
    createClient: () => new FakeClient()
  });
  await controller.start();

  for (let index = 0; index < 10_000; index += 1) {
    controller.createEditCommand(`Action ${index}`, { reciteVersionGuard: () => true });
  }
  assert.ok(controller.editCommands.commands.size <= 1);
  assert.ok(controller.editCommands.retired.size <= 1);
  const beforeExpiry = controller.createEditCommand("Before expiry", {
    reciteVersionGuard: () => true
  });
  controller.createEditCommand("Current", { reciteVersionGuard: () => true });
  await new Promise((resolve) => setTimeout(resolve, 30));
  assert.equal(await api.commands.executeCommand(
    beforeExpiry.command, ...beforeExpiry.arguments
  ), false);
  assert.equal(messages.at(-1), "Recite code action is no longer available.");
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
  const controller = new ExtensionController(api, output(messages), { delete() {} }, { createClient: () => new FakeClient() });
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
  const controller = new ExtensionController(api, output(messages), { delete() {} }, { createClient: () => new FakeClient() });
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
