import test from "node:test";
import assert from "node:assert/strict";
import { createUserInterface } from "../src/user-interface.js";

test("semantic UI operations localize source messages and keep the channel private", () => {
  const lines = [];
  const raw = [];
  let disposed = false;
  const api = {
    l10n: { t: (template, detail) => `[${template}] ${detail}` },
    window: {
      createOutputChannel: (name) => {
        assert.equal(name, "[Recite] undefined");
        return {
          append: (value) => raw.push(value),
          appendLine: (value) => lines.push(value),
          dispose: () => { disposed = true; }
        };
      }
    }
  };
  const ui = createUserInterface(api);

  ui.serverStartFailed("failed");
  ui.serverStderr("stderr");
  ui.serverNotification("notification");
  assert.deepEqual(lines, [
    "[Recite language server could not be started: {0}.] failed",
    "notification"
  ]);
  assert.deepEqual(raw, ["stderr"]);
  assert.equal(ui.output, undefined);
  assert.equal(ui.write, undefined);
  assert.equal(ui.show, undefined);
  assert.match(ui.configurationPathInvalid().message, /recite\.lsp\.path/);
  assert.match(ui.serverNotRunning().message, /language server is not running/);

  ui.dispose();
  assert.equal(disposed, true);
});
