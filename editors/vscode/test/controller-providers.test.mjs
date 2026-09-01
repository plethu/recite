import test from "node:test";
import assert from "node:assert/strict";
import { ExtensionController } from "../src/controller.js";
import { FakeClient, action, hostApi, output, uri } from "./controller-fixtures.mjs";

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

test("code-action provider projects commands without a VS Code Command constructor", async () => {
  const primary = {
    languageId: "recite",
    version: 4,
    uri: uri("dialogue.recite"),
    getText: () => ":: dialogue"
  };
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  delete api.Command;
  api.workspace.textDocuments.push(primary);
  const controller = new ExtensionController(api, output(), { delete() {} }, {
    createClient: () => {
      const client = new FakeClient();
      client.request = async () => [{
        title: "Run command",
        command: "recite.run",
        arguments: ["dialogue.recite", 2]
      }];
      return client;
    }
  });
  await controller.start();
  const provider = api.registeredProviders.find(({ name }) => name === "code-actions").provider;
  const commandOnly = await provider.provideCodeActions(
    primary,
    new api.Range(new api.Position(0, 0), new api.Position(0, 2)),
    { diagnostics: [] }
  );
  assert.equal(commandOnly.length, 1);
  assert.deepEqual(commandOnly[0], {
    title: "Run command",
    command: "recite.run",
    arguments: ["dialogue.recite", 2]
  });
  assert.equal(Object.getPrototypeOf(commandOnly[0]), Object.prototype);

  controller.client.request = async () => [
    { title: "Run command", command: "recite.run" },
    { title: "Unavailable fix", kind: "quickfix", disabled: { reason: "not ready" } },
    { title: "Run with arguments", command: "recite.run", arguments: ["other.recite"] }
  ];
  const mixed = await provider.provideCodeActions(
    primary,
    new api.Range(new api.Position(0, 0), new api.Position(0, 2)),
    { diagnostics: [] }
  );
  assert.equal(mixed.length, 3);
  assert.deepEqual(mixed[0], { title: "Run command", command: "recite.run" });
  assert.equal(mixed[1] instanceof api.CodeAction, true);
  assert.equal(mixed[1].title, "Unavailable fix");
  assert.equal(mixed[1].kind, "quickfix");
  assert.deepEqual(mixed[1].disabled, { reason: "not ready" });
  assert.deepEqual(mixed[2], {
    title: "Run with arguments",
    command: "recite.run",
    arguments: ["other.recite"]
  });
  assert.equal(Object.hasOwn(mixed[0], "arguments"), false);
  await controller.dispose();
});
