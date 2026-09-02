import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { initializeParams, readConfiguration } from "../src/configuration.js";

test("configuration resolves project-relative binaries without a shell", () => {
  const api = fakeApi({
    workspaceFolders: [{ name: "demo", uri: { fsPath: "/workspace/demo", toString: () => "file:///workspace/demo" } }],
    values: {
      "lsp.path": "./tools/recite-lsp",
      "lsp.args": ["--local"],
      "lsp.projectRoot": "project"
    }
  });

  assert.deepEqual(readConfiguration(api, userInterface()), {
    command: path.resolve("/workspace/demo/project", "tools/recite-lsp"),
    args: ["--local"],
    cwd: path.resolve("/workspace/demo/project"),
    projectRoot: path.resolve("/workspace/demo/project"),
    projectRootOverridden: true
  });
});

test("initialization advertises UTF-16, full sync, and dynamic watch registration", () => {
  const api = fakeApi({
    workspaceFolders: [{ name: "demo", uri: { fsPath: "/workspace/demo", toString: () => "file:///workspace/demo" } }]
  });
  const params = initializeParams(api, "/workspace/demo", true);

  assert.equal(params.rootUri, "file:///workspace/demo");
  assert.deepEqual(params.workspaceFolders, [{ name: "demo", uri: "file:///workspace/demo" }]);
  assert.deepEqual(params.capabilities.general.positionEncodings, ["utf-16"]);
  assert.deepEqual(params.capabilities.workspace.didChangeWatchedFiles, { dynamicRegistration: true });
  assert.equal(params.capabilities.textDocument.synchronization.didSave, true);
  assert.equal(params.capabilities.textDocument.rename, undefined);
});

test("relative project roots require a workspace folder", () => {
  const api = fakeApi({ workspaceFolders: [], values: { "lsp.projectRoot": "project" } });
  assert.throws(() => readConfiguration(api, userInterface()), /needs a workspace/);
});

test("configuration validation reports canonical localized messages", () => {
  assert.throws(
    () => readConfiguration(fakeApi({ workspaceFolders: [], values: { "lsp.path": "" } }), userInterface()),
    /recite\.lsp\.path must be a non-empty string/
  );
  assert.throws(
    () => readConfiguration(fakeApi({ workspaceFolders: [], values: { "lsp.args": ["--ok", 1] } }), userInterface()),
    /recite\.lsp\.args must be an array of strings/
  );
  assert.throws(
    () => readConfiguration(fakeApi({ workspaceFolders: [], values: { "lsp.projectRoot": 1 } }), userInterface()),
    /recite\.lsp\.projectRoot must be a string/
  );
});

function fakeApi({ workspaceFolders, values = {} }) {
  return {
    workspace: {
      workspaceFolders,
      getConfiguration: () => ({ get: (key, fallback) => values[key] ?? fallback })
    },
    Uri: {
      file: (value) => ({ toString: () => `file://${value}` })
    }
  };
}

function userInterface() {
  return {
    configurationPathInvalid: () => new Error("recite.lsp.path must be a non-empty string."),
    configurationArgsInvalid: () => new Error("recite.lsp.args must be an array of strings."),
    configurationProjectRootInvalid: () => new Error("recite.lsp.projectRoot must be a string."),
    configurationProjectRootNeedsWorkspace: () =>
      new Error("recite.lsp.projectRoot needs a workspace for relative paths.")
  };
}
