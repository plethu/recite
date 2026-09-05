import test from "node:test";
import assert from "node:assert/strict";
import { RenameCommand } from "../src/rename-command.js";
import { document, harness, range, workspaceEdit } from "./rename-test-fixtures.mjs";

test("explicit rename captures the client and applies a current versioned workspace edit", async () => {
  const h = harness();
  const requests = [];
  let promptOptions;
  h.ui.chooseRenameName = async (placeholder) => {
    promptOptions = placeholder;
    return "renamed";
  };
  h.client.request = async (method, params) => {
    requests.push({ method, params });
    if (method === "textDocument/prepareRename") return { range: range(), placeholder: "work" };
    return workspaceEdit(h.primary, h.sibling);
  };
  const command = new RenameCommand(h.api, h.ui, () => h.client);

  assert.equal(await command.execute(), true);
  assert.equal(promptOptions, "work");
  assert.deepEqual(requests.map(({ method }) => method), [
    "textDocument/prepareRename", "textDocument/rename"
  ]);
  assert.deepEqual(requests[0].params, {
    textDocument: { uri: h.primary.uri.toString() },
    position: { line: 1, character: 4 }
  });
  assert.equal(h.applied.length, 1);
  assert.equal(h.applied[0].replacements.length, 2);
});

test("rename input cancellation is a no-op and does not send rename", async () => {
  const h = harness();
  const requests = [];
  h.client.request = async (method) => {
    requests.push(method);
    return method === "textDocument/prepareRename" ? { defaultBehavior: true } : undefined;
  };
  h.ui.chooseRenameName = async () => undefined;
  const command = new RenameCommand(h.api, h.ui, () => h.client);

  assert.equal(await command.execute(), undefined);
  assert.deepEqual(requests, ["textDocument/prepareRename"]);
  assert.deepEqual(h.applied, []);
  assert.deepEqual(h.messages, []);
});

test("rename accepts only the three LSP prepareRename response shapes", async (t) => {
  const valid = [
    ["bare range", range()],
    ["range with placeholder", { range: range(), placeholder: "work" }],
    ["default behavior", { defaultBehavior: true }]
  ];
  for (const [label, prepared] of valid) {
    await t.test(label, async () => {
      const h = harness();
      h.client.request = async (method) => method === "textDocument/prepareRename"
        ? prepared
        : workspaceEdit(h.primary, h.sibling);
      h.ui.chooseRenameName = async () => "renamed";
      const command = new RenameCommand(h.api, h.ui, () => h.client);
      assert.equal(await command.execute(), true);
      assert.equal(h.applied.length, 1);
    });
  }
  const invalid = [
    ["missing placeholder", { range: range() }],
    ["non-string placeholder", { range: range(), placeholder: 1 }],
    ["reversed bare range", { start: { line: 2, character: 0 }, end: { line: 1, character: 9 } }],
    ["reversed range result", {
      range: { start: { line: 2, character: 0 }, end: { line: 1, character: 9 } },
      placeholder: "work"
    }],
    ["false default behavior", { defaultBehavior: false }],
    ["mixed range and default behavior", { range: range(), defaultBehavior: true }],
    ["extra range result field", { range: range(), placeholder: "work", extra: true }],
    ["extra bare range field", { ...range(), extra: true }],
    ["default behavior with extra field", { defaultBehavior: true, placeholder: "work" }]
  ];
  for (const [label, prepared] of invalid) {
    await t.test(label, async () => {
      const h = harness();
      h.client.request = async () => prepared;
      const command = new RenameCommand(h.api, h.ui, () => h.client);
      assert.equal(await command.execute(), false);
      assert.deepEqual(h.applied, []);
      assert.deepEqual(h.messages, ["invalid"]);
    });
  }
});

test("rename refuses null and malformed prepare responses", async (t) => {
  for (const prepared of [null, { range: { start: { line: 0 } } }]) {
    await t.test(JSON.stringify(prepared), async () => {
      const h = harness();
      h.client.request = async () => prepared;
      const command = new RenameCommand(h.api, h.ui, () => h.client);
      assert.equal(await command.execute(), false);
      assert.deepEqual(h.applied, []);
      assert.equal(h.messages.length, 1);
      assert.equal(h.messages[0], prepared === null ? "unavailable" : "invalid");
    });
  }
});

test("rename rejects an unversioned or closed sibling before apply", async (t) => {
  for (const [label, siblingChange, removeSibling] of [
    ["unversioned", { textDocument: { uri: "file:///sibling.recite" }, edits: [] }, false],
    ["closed", { textDocument: { uri: "file:///sibling.recite", version: 3 }, edits: [] }, true]
  ]) {
    await t.test(label, async () => {
      const h = harness();
      h.client.request = async (method) => method === "textDocument/prepareRename"
        ? { defaultBehavior: true }
        : { documentChanges: [workspaceEdit(h.primary, h.sibling).documentChanges[0], siblingChange] };
      if (removeSibling) h.documents.splice(1, 1);
      h.ui.chooseRenameName = async () => "renamed";
      const command = new RenameCommand(h.api, h.ui, () => h.client);
      assert.equal(await command.execute(), false);
      assert.deepEqual(h.applied, []);
      assert.deepEqual(h.messages, ["unavailable"]);
    });
  }
});

test("rename rejects mixed top-level changes even when documentChanges is valid", async () => {
  const h = harness();
  h.documents.splice(1, 1);
  h.client.request = async (method) => {
    if (method === "textDocument/prepareRename") return { defaultBehavior: true };
    return {
      documentChanges: [workspaceEdit(h.primary, h.sibling).documentChanges[0]],
      changes: {
        [h.sibling.uri.toString()]: [{ range: range(), newText: "renamed" }]
      }
    };
  };
  h.ui.chooseRenameName = async () => "renamed";
  const command = new RenameCommand(h.api, h.ui, () => h.client);

  assert.equal(await command.execute(), false);
  assert.deepEqual(h.applied, []);
  assert.deepEqual(h.messages, ["invalid"]);
});

test("rename rejects reversed ranges in versioned document changes", async () => {
  const h = harness();
  h.client.request = async (method) => {
    if (method === "textDocument/prepareRename") return { defaultBehavior: true };
    const edit = workspaceEdit(h.primary, h.sibling);
    edit.documentChanges[0].edits[0].range = {
      start: { line: 2, character: 0 },
      end: { line: 1, character: 9 }
    };
    return edit;
  };
  h.ui.chooseRenameName = async () => "renamed";
  const command = new RenameCommand(h.api, h.ui, () => h.client);

  assert.equal(await command.execute(), false);
  assert.deepEqual(h.applied, []);
  assert.deepEqual(h.messages, ["unavailable"]);
});

test("rename checks document generations immediately before apply", async () => {
  const h = harness();
  h.client.request = async (method) => method === "textDocument/prepareRename"
    ? { defaultBehavior: true }
    : workspaceEdit(h.primary, h.sibling);
  h.ui.chooseRenameName = async () => "renamed";
  h.api.workspace.applyEdit = async (edit) => {
    h.applied.push(edit);
    h.primary.version = 2;
    return true;
  };
  const command = new RenameCommand(h.api, h.ui, () => h.client);

  // The host mutation occurs only once applyEdit is called. The guarded
  // WorkspaceEdit has already passed the status boundary and the host owns
  // whether it accepts the edit transaction.
  assert.equal(await command.execute(), true);
  assert.equal(h.applied.length, 1);
});

test("rename refuses a sibling that becomes stale or reopens before apply", async (t) => {
  for (const [label, mutate] of [
    ["stale", (h) => { h.sibling.version = 4; }],
    ["reopened", (h) => {
      h.documents[1] = document("sibling.recite", h.sibling.version);
    }]
  ]) {
    await t.test(label, async () => {
      const h = harness();
      h.client.request = async (method) => method === "textDocument/prepareRename"
        ? { defaultBehavior: true }
        : workspaceEdit(h.primary, h.sibling);
      h.ui.chooseRenameName = async () => "renamed";
      let lookups = 0;
      const command = new RenameCommand(h.api, h.ui, () => h.client, {
        getOpenDocument: (uri) => {
          const found = h.documents.find((candidate) => candidate.uri.toString() === uri.toString());
          // Two lookups occur while translating the edit; the first status
          // lookup is where a close/reopen or edit race is introduced.
          if (++lookups === 3) mutate(h);
          return found;
        }
      });
      assert.equal(await command.execute(), false);
      assert.deepEqual(h.applied, []);
      assert.deepEqual(h.messages, ["stale"]);
    });
  }
});

test("rename reports a host apply rejection", async () => {
  const h = harness();
  h.client.request = async (method) => method === "textDocument/prepareRename"
    ? { defaultBehavior: true }
    : workspaceEdit(h.primary, h.sibling);
  h.ui.chooseRenameName = async () => "renamed";
  h.api.workspace.applyEdit = async () => false;
  const command = new RenameCommand(h.api, h.ui, () => h.client);

  assert.equal(await command.execute(), false);
  assert.deepEqual(h.messages, ["apply-failed"]);
});

test("rename reports a host apply exception", async () => {
  const h = harness();
  h.client.request = async (method) => method === "textDocument/prepareRename"
    ? { defaultBehavior: true }
    : workspaceEdit(h.primary, h.sibling);
  h.ui.chooseRenameName = async () => "renamed";
  h.api.workspace.applyEdit = async () => { throw new Error("host rejected"); };
  const command = new RenameCommand(h.api, h.ui, () => h.client);

  assert.equal(await command.execute(), false);
  assert.deepEqual(h.messages, ["apply-failed"]);
});

test("rename reports prepare and rename request failures", async (t) => {
  await t.test("prepare", async () => {
    const h = harness();
    h.client.request = async () => { throw new Error("prepare failed"); };
    const command = new RenameCommand(h.api, h.ui, () => h.client);
    assert.equal(await command.execute(), false);
    assert.deepEqual(h.messages, [["request-failed", "prepare failed"]]);
  });
  await t.test("rename", async () => {
    const h = harness();
    h.client.request = async (method) => {
      if (method === "textDocument/prepareRename") return { defaultBehavior: true };
      throw new Error("rename failed");
    };
    h.ui.chooseRenameName = async () => "renamed";
    const command = new RenameCommand(h.api, h.ui, () => h.client);
    assert.equal(await command.execute(), false);
    assert.deepEqual(h.messages, [["request-failed", "rename failed"]]);
  });
});
