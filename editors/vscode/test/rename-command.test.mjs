import test from "node:test";
import assert from "node:assert/strict";
import { RenameCommand } from "../src/rename-command.js";
import { createUserInterface } from "../src/user-interface.js";

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
    if (method === "textDocument/prepareRename") {
      return { range: range(), placeholder: "work" };
    }
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

function harness() {
  const primary = document("dialogue.recite", 4);
  const sibling = document("sibling.recite", 3);
  const documents = [primary, sibling];
  const applied = [];
  const messages = [];
  const client = { status: "running", request: async () => undefined };
  const api = {
    workspace: {
      isTrusted: true,
      textDocuments: documents,
      applyEdit: async (edit) => { applied.push(edit); return true; }
    },
    window: { activeTextEditor: { document: primary, selection: { active: { line: 1, character: 4 } } } },
    Uri: { parse: (value) => ({ toString: () => value }) },
    Position: class Position {
      constructor(line, character) { this.line = line; this.character = character; }
    },
    Range: class Range {
      constructor(start, end) { this.start = start; this.end = end; }
    },
    WorkspaceEdit: class WorkspaceEdit {
      constructor() { this.replacements = []; }
      replace(uri, editRange, newText) { this.replacements.push({ uri, editRange, newText }); }
    }
  };
  const ui = {
    activeEditor: () => api.window.activeTextEditor,
    documentIsOpen: (document_) => documents.includes(document_),
    chooseRenameName: async () => "renamed",
    commandNotTrusted: () => messages.push("untrusted"),
    renameBusy: () => messages.push("busy"),
    renameDocumentRequired: () => messages.push("document"),
    renameUnavailable: () => messages.push("unavailable"),
    renameInvalid: () => messages.push("invalid"),
    renameStale: () => messages.push("stale"),
    renameApplyFailed: () => messages.push("apply-failed"),
    renameRequestFailed: (detail) => messages.push(["request-failed", detail]),
    serverNotRunning: () => new Error("Recite language server is not running."),
    commandFailure: (detail) => messages.push(["failure", detail])
  };
  return { api, ui, client, primary, sibling, documents, applied, messages };
}

function document(file, version) {
  return {
    languageId: "recite",
    version,
    uri: { toString: () => `file:///${file}` },
    getText: () => `:: ${file}`
  };
}

function range() {
  return { start: { line: 1, character: 0 }, end: { line: 1, character: 4 } };
}

function workspaceEdit(primary, sibling) {
  return {
    documentChanges: [
      {
        textDocument: { uri: primary.uri.toString(), version: primary.version },
        edits: [{ range: range(), newText: "renamed" }]
      },
      {
        textDocument: { uri: sibling.uri.toString(), version: sibling.version },
        edits: [{ range: range(), newText: "renamed" }]
      }
    ]
  };
}
