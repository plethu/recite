import test from "node:test";
import assert from "node:assert/strict";
import { createUserInterface } from "../src/user-interface.js";

test("semantic UI operations localize source messages and keep the channel private", () => {
  const lines = [];
  const raw = [];
  const notices = [];
  let disposed = false;
  const api = {
    l10n: { t: (template, detail) => detail === undefined ? `[${template}]` : `[${template}] ${detail}` },
    window: {
      createOutputChannel: (name) => {
        assert.equal(name, "[Recite]");
        return {
          append: (value) => raw.push(value),
          appendLine: (value) => lines.push(value),
          dispose: () => { disposed = true; }
        };
      },
      showErrorMessage: (value) => notices.push(["error", value]),
      showWarningMessage: (value) => notices.push(["warning", value]),
      showInformationMessage: (value) => notices.push(["information", value])
    }
  };
  const ui = createUserInterface(api);

  ui.serverTransportFailure("EPIPE");
  ui.serverProtocolFailure();
  ui.serverLifecycleFailure("ENOENT");
  ui.serverExited();
  ui.restartScheduled(1_234);
  ui.restartExhausted();
  ui.serverStderr("stderr");
  ui.serverLogMessage("log");
  ui.serverErrorMessage("error from server");
  ui.serverWarningMessage("warning from server");
  ui.serverInfoMessage("info from server");
  assert.deepEqual(lines, [
    "[Recite language server transport failed: {0}.] EPIPE",
    "[Recite language server protocol failed.]",
    "[Recite language server lifecycle failed: {0}.] ENOENT",
    "[Recite language server exited.]",
    "[Recite language server restart scheduled in {0} milliseconds.] 1234",
    "[Recite language server restart attempts exhausted.]",
    "log",
    "error from server",
    "warning from server",
    "info from server"
  ]);
  assert.deepEqual(raw, ["stderr"]);
  assert.deepEqual(notices, [
    ["error", "[Recite language server transport failed: {0}.] EPIPE"],
    ["error", "[Recite language server protocol failed.]"],
    ["error", "[Recite language server lifecycle failed: {0}.] ENOENT"],
    ["error", "[Recite language server exited.]"],
    ["error", "[Recite language server restart attempts exhausted.]"],
    ["error", "error from server"],
    ["warning", "warning from server"],
    ["information", "info from server"]
  ]);
  assert.equal(ui.output, undefined);
  assert.equal(ui.write, undefined);
  assert.equal(ui.show, undefined);
  assert.match(ui.configurationPathInvalid().message, /recite\.lsp\.path/);
  assert.match(ui.serverNotRunning().message, /language server is not running/);

  ui.dispose();
  assert.equal(disposed, true);
});

test("runtime input pickers localize prompts and constrain file types", async () => {
  const calls = [];
  const api = {
    l10n: { t: (template) => template },
    window: {
      createOutputChannel: () => ({ appendLine() {}, dispose() {} }),
      showOpenDialog: async (options) => { calls.push(["open", options]); return undefined; },
      showInputBox: async (options) => { calls.push(["input", options]); return undefined; }
    }
  };
  const ui = createUserInterface(api);
  await ui.chooseAssetPath();
  await ui.chooseBlock();
  await ui.chooseFixturePath();
  assert.deepEqual(calls, [
    ["open", {
      title: "Choose the compiled Recite asset",
      filters: { "Recite compiled assets": ["recitec"] },
      canSelectFiles: true,
      canSelectFolders: false,
      canSelectMany: false
    }],
    ["input", {
      title: "Enter the Recite block name",
      prompt: "Block name used by the fixture",
      placeHolder: "For example, start"
    }],
    ["open", {
      title: "Choose the runtime fixture",
      filters: { "Recite runtime fixtures": ["toml"] },
      canSelectFiles: true,
      canSelectFolders: false,
      canSelectMany: false
    }]
  ]);
  ui.dispose();
});
