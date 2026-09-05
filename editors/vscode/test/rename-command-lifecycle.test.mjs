import test from "node:test";
import assert from "node:assert/strict";
import { RenameCommand } from "../src/rename-command.js";
import { createUserInterface } from "../src/user-interface.js";
import { harness, workspaceEdit } from "./rename-test-fixtures.mjs";

test("rename revalidates the active document after prepare and refuses stale edits", async () => {
  const h = harness();
  let resolvePrepare;
  h.client.request = () => new Promise((resolve) => { resolvePrepare = resolve; });
  const command = new RenameCommand(h.api, h.ui, () => h.client);
  const pending = command.execute();
  h.primary.version = 2;
  resolvePrepare({ defaultBehavior: true });

  assert.equal(await pending, false);
  assert.deepEqual(h.applied, []);
  assert.deepEqual(h.messages, ["stale"]);
});

test("rename fences cursor movement while prepareRename is in flight", async () => {
  const h = harness();
  let resolvePrepare;
  h.client.request = () => new Promise((resolve) => { resolvePrepare = resolve; });
  const command = new RenameCommand(h.api, h.ui, () => h.client);
  const pending = command.execute();
  h.api.window.activeTextEditor.selection.active = { line: 1, character: 5 };
  resolvePrepare({ defaultBehavior: true });

  assert.equal(await pending, false);
  assert.deepEqual(h.applied, []);
  assert.deepEqual(h.messages, ["stale"]);
});

test("rename treats the active document closing during prepareRename as stale", async () => {
  const h = harness();
  let resolvePrepare;
  h.client.request = () => new Promise((resolve) => { resolvePrepare = resolve; });
  const command = new RenameCommand(h.api, h.ui, () => h.client);
  const pending = command.execute();
  h.api.window.activeTextEditor = undefined;
  resolvePrepare({ defaultBehavior: true });

  assert.equal(await pending, false);
  assert.deepEqual(h.applied, []);
  assert.deepEqual(h.messages, ["stale"]);
});

test("rename fences cursor movement while the localized prompt is in flight", async () => {
  const h = harness();
  let resolvePrompt;
  h.client.request = async (method) => method === "textDocument/prepareRename"
    ? { defaultBehavior: true }
    : workspaceEdit(h.primary, h.sibling);
  h.ui.chooseRenameName = () => new Promise((resolve) => { resolvePrompt = resolve; });
  const command = new RenameCommand(h.api, h.ui, () => h.client);
  const pending = command.execute();
  await Promise.resolve();
  h.api.window.activeTextEditor.selection.active = { line: 2, character: 1 };
  resolvePrompt("renamed");

  assert.equal(await pending, false);
  assert.deepEqual(h.applied, []);
  assert.deepEqual(h.messages, ["stale"]);
});

test("rename fences cursor movement while the rename response is in flight", async () => {
  const h = harness();
  let resolveRename;
  h.client.request = (method) => {
    if (method === "textDocument/prepareRename") return { defaultBehavior: true };
    return new Promise((resolve) => { resolveRename = resolve; });
  };
  h.ui.chooseRenameName = async () => "renamed";
  const command = new RenameCommand(h.api, h.ui, () => h.client);
  const pending = command.execute();
  await new Promise((resolve) => setImmediate(resolve));
  h.api.window.activeTextEditor.selection.active = { line: 3, character: 2 };
  resolveRename(workspaceEdit(h.primary, h.sibling));

  assert.equal(await pending, false);
  assert.deepEqual(h.applied, []);
  assert.deepEqual(h.messages, ["stale"]);
});

test("rename revalidates the exact client after a restart", async () => {
  const h = harness();
  let resolvePrepare;
  h.client.request = () => new Promise((resolve) => { resolvePrepare = resolve; });
  let currentClient = h.client;
  const command = new RenameCommand(h.api, h.ui, () => currentClient);
  const pending = command.execute();
  currentClient = { status: "running", request: async () => ({ defaultBehavior: true }) };
  resolvePrepare({ defaultBehavior: true });

  assert.equal(await pending, false);
  assert.deepEqual(h.applied, []);
  assert.deepEqual(h.messages, ["stale"]);
});

test("rename refuses a client replacement while the prompt is in flight", async () => {
  const h = harness();
  let resolvePrompt;
  let currentClient = h.client;
  h.client.request = async (method) => method === "textDocument/prepareRename"
    ? { defaultBehavior: true }
    : workspaceEdit(h.primary, h.sibling);
  h.ui.chooseRenameName = () => new Promise((resolve) => { resolvePrompt = resolve; });
  const command = new RenameCommand(h.api, h.ui, () => currentClient);
  const pending = command.execute();
  await Promise.resolve();
  currentClient = { status: "running", request: async () => workspaceEdit(h.primary, h.sibling) };
  resolvePrompt("renamed");

  assert.equal(await pending, false);
  assert.deepEqual(h.applied, []);
  assert.deepEqual(h.messages, ["stale"]);
});

test("rename refuses a client replacement while the rename response is in flight", async () => {
  const h = harness();
  let resolveRename;
  let currentClient = h.client;
  h.client.request = (method) => {
    if (method === "textDocument/prepareRename") return { defaultBehavior: true };
    return new Promise((resolve) => { resolveRename = resolve; });
  };
  h.ui.chooseRenameName = async () => "renamed";
  const command = new RenameCommand(h.api, h.ui, () => currentClient);
  const pending = command.execute();
  await new Promise((resolve) => setImmediate(resolve));
  currentClient = { status: "running", request: async () => workspaceEdit(h.primary, h.sibling) };
  resolveRename(workspaceEdit(h.primary, h.sibling));

  assert.equal(await pending, false);
  assert.deepEqual(h.applied, []);
  assert.deepEqual(h.messages, ["stale"]);
});

test("disposal during applyEdit suppresses every post-disposal outcome", async (t) => {
  for (const outcome of ["true", "false", "reject"]) {
    await t.test(outcome, async () => {
      const h = harness();
      let resolveApply;
      let rejectApply;
      let applyStarted = false;
      h.client.request = async (method) => method === "textDocument/prepareRename"
        ? { defaultBehavior: true }
        : workspaceEdit(h.primary, h.sibling);
      h.ui.chooseRenameName = async () => "renamed";
      h.api.workspace.applyEdit = () => {
        applyStarted = true;
        return new Promise((resolve, reject) => {
          resolveApply = resolve;
          rejectApply = reject;
        });
      };
      const command = new RenameCommand(h.api, h.ui, () => h.client);
      const pending = command.execute();
      await new Promise((resolve) => setImmediate(resolve));
      assert.equal(applyStarted, true);
      command.dispose();
      if (outcome === "reject") rejectApply(new Error("late host failure"));
      else resolveApply(outcome === "true");

      assert.equal(await pending, false);
      assert.deepEqual(h.messages, []);
    });
  }
});

test("disposing an in-flight rename suppresses stale UI and apply", async () => {
  const h = harness();
  let resolvePrepare;
  h.client.request = () => new Promise((resolve) => { resolvePrepare = resolve; });
  const command = new RenameCommand(h.api, h.ui, () => h.client);
  const pending = command.execute();
  command.dispose();
  resolvePrepare({ defaultBehavior: true });

  assert.equal(await pending, false);
  assert.deepEqual(h.applied, []);
  assert.deepEqual(h.messages, []);
});

test("a stopped server remains visible through the production UI while preserving its Error", async () => {
  const h = harness();
  const notices = [];
  const lines = [];
  const api = {
    ...h.api,
    l10n: {
      t(template, detail) {
        return detail === undefined ? template : template.replace("{0}", String(detail));
      }
    },
    window: {
      ...h.api.window,
      createOutputChannel: () => ({ append() {}, appendLine: (value) => lines.push(value), dispose() {} }),
      showErrorMessage: (value) => notices.push(value),
      showWarningMessage() {},
      showInformationMessage() {}
    }
  };
  const ui = createUserInterface(api);
  const command = new RenameCommand(api, ui, () => undefined);

  assert.equal(await command.execute(), false);
  assert.equal(notices.length, 1);
  assert.match(notices[0], /language server is not running/);
  assert.equal(lines.length, 1);
  assert.match(lines[0], /command failed:.*language server is not running/);
  ui.dispose();
});

test("overlapping rename commands are refused while the first waits", async () => {
  const h = harness();
  let resolvePrompt;
  h.client.request = async (method) => method === "textDocument/prepareRename"
    ? { defaultBehavior: true }
    : workspaceEdit(h.primary, h.sibling);
  h.ui.chooseRenameName = () => new Promise((resolve) => { resolvePrompt = resolve; });
  const command = new RenameCommand(h.api, h.ui, () => h.client);
  const first = command.execute();
  assert.equal(await command.execute(), false);
  assert.deepEqual(h.messages, ["busy"]);
  resolvePrompt("renamed");
  assert.equal(await first, true);
});
