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

  assert.deepEqual(readConfiguration(api), {
    command: path.resolve("/workspace/demo/project", "tools/recite-lsp"),
    args: ["--local"],
    cwd: path.resolve("/workspace/demo/project"),
    projectRoot: path.resolve("/workspace/demo/project")
  });
});

test("initialization advertises UTF-16 and full sync without claiming watch support", () => {
  const api = fakeApi({
    workspaceFolders: [{ name: "demo", uri: { fsPath: "/workspace/demo", toString: () => "file:///workspace/demo" } }]
  });
  const params = initializeParams(api, "/workspace/demo");

  assert.equal(params.rootUri, "file:///workspace/demo");
  assert.deepEqual(params.capabilities.general.positionEncodings, ["utf-16"]);
  assert.equal(params.capabilities.workspace.didChangeWatchedFiles, undefined);
  assert.equal(params.capabilities.textDocument.synchronization.didSave, true);
});

test("relative project roots require a workspace folder", () => {
  const api = fakeApi({ workspaceFolders: [], values: { "lsp.projectRoot": "project" } });
  assert.throws(() => readConfiguration(api), /needs a workspace/);
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
