import test from "node:test";
import assert from "node:assert/strict";
import Module from "node:module";
import { createRequire } from "node:module";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { hostApi } from "./controller-fixtures.mjs";

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const manifest = JSON.parse(await readFile(path.join(packageRoot, "package.json"), "utf8"));
const main = path.resolve(packageRoot, manifest.main);
const implementationPath = path.join(packageRoot, "dist", "extension.js");
const require = createRequire(import.meta.url);

test("the CommonJS entry injects the host API into the built ESM lifecycle", async () => {
  const api = hostApi({
    isTrusted: () => false,
    onDidGrantWorkspaceTrust: () => ({ dispose() {} })
  });
  const outputChannels = [];
  api.window = {
    createOutputChannel(name) {
      const channel = {
        name,
        append() {},
        appendLine() {},
        dispose() { channel.disposed = true; },
        disposed: false
      };
      outputChannels.push(channel);
      return channel;
    },
    showErrorMessage() {},
    showWarningMessage() {},
    showInformationMessage() {}
  };
  api.languages.createDiagnosticCollection = () => ({
    delete() {},
    dispose() {},
    set() {}
  });

  const originalLoad = Module._load;
  let vscodeLoadCount = 0;
  Module._load = function load(request, parent, isMain) {
    if (request === "vscode") {
      vscodeLoadCount += 1;
      return api;
    }
    return originalLoad.call(this, request, parent, isMain);
  };

  try {
    const entry = require(main);
    assert.equal(vscodeLoadCount, 1, "the CommonJS entry must require vscode exactly once");
    assert.equal(typeof entry.activate, "function");
    assert.equal(typeof entry.deactivate, "function");

    const context = { subscriptions: [] };
    await entry.activate(context);
    assert.equal(context.subscriptions.length, 3);
    const controller = context.subscriptions[2];
    assert.equal(controller.api, api);
    assert.equal(controller.disposed, false);
    assert.equal(outputChannels.length, 1);

    await entry.deactivate();
    assert.equal(controller.disposed, true,
      "deactivate must reach the controller created by the same ESM module");

    const implementation = await import(pathToFileURL(implementationPath).href);
    assert.equal(typeof implementation.activateWithVscode, "function");
    assert.equal(typeof implementation.deactivateWithVscode, "function");

    const failure = new Error("host activation sentinel");
    api.window.createOutputChannel = () => { throw failure; };
    await assert.rejects(entry.activate({ subscriptions: [] }), (error) => error === failure);
  } finally {
    Module._load = originalLoad;
  }
});
