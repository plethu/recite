import test from "node:test";
import assert from "node:assert/strict";
import { ExtensionController } from "../src/controller.js";
import { FakeClient, hostApi, output, waitFor } from "./controller-fixtures.mjs";

test("a configuration restart reopens the exhausted recovery budget", async () => {
  let configurationChanged;
  const latest = await runExhaustedRestartScenario({
    installTrigger: (api, callback) => {
      api.workspace.onDidChangeConfiguration = (listener) => {
        configurationChanged = listener;
        return { dispose() {} };
      };
    },
    updateAuthority: ({ settings }) => { settings.path = "latest-config"; },
    trigger: () => configurationChanged({ affectsConfiguration: () => true }),
    assertAuthority: ({ starts }) => {
      assert.equal(starts[3].configuration.command, "latest-config");
      assert.equal(starts[4].configuration.command, "latest-config");
    }
  });

  assert.equal(latest.scheduled.length, 3, "the explicit restart should get a fresh bounded retry");
  assert.equal(latest.exhausted.length, 1, "a successful retry must not repeat the exhaustion notice");
});

test("a workspace-folder restart reopens the exhausted recovery budget", async () => {
  let folderChanged;
  const first = folder("first");
  const second = folder("second");
  const latest = await runExhaustedRestartScenario({
    initialFolders: [first],
    installTrigger: (api, callback) => {
      api.workspace.onDidChangeWorkspaceFolders = (listener) => {
        folderChanged = listener;
        return { dispose() {} };
      };
    },
    updateAuthority: ({ api }) => { api.workspace.workspaceFolders = [first, second]; },
    trigger: () => folderChanged({ added: [second], removed: [] }),
    assertAuthority: ({ starts }) => {
      assert.deepEqual(starts[3].params.workspaceFolders, [
        { name: "first", uri: "file:///workspace/first" },
        { name: "second", uri: "file:///workspace/second" }
      ]);
      assert.deepEqual(starts[4].params.workspaceFolders, starts[3].params.workspaceFolders);
    }
  });

  assert.equal(latest.scheduled.length, 3, "the folder change should use the fresh bounded retry");
  assert.equal(latest.exhausted.length, 1, "a successful retry must not repeat the exhaustion notice");
});

async function runExhaustedRestartScenario({
  initialFolders = [],
  installTrigger,
  updateAuthority,
  trigger,
  assertAuthority
}) {
  const messages = [];
  const settings = { path: "initial", args: [], projectRoot: "" };
  const starts = [];
  const clients = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  api.workspace.workspaceFolders = initialFolders;
  api.workspace.getConfiguration = () => ({
    get: (key, fallback) => ({
      "lsp.path": settings.path,
      "lsp.args": settings.args,
      "lsp.projectRoot": settings.projectRoot
    }[key] ?? fallback)
  });
  installTrigger(api, trigger);
  const controller = new ExtensionController(api, output(messages), { delete() {} }, {
    restartDelaysMs: [0, 0],
    stableRunMs: 100,
    createClient: (configuration) => {
      const index = clients.length;
      const client = new FakeClient();
      clients.push(client);
      const start = client.start.bind(client);
      client.start = async (params) => {
        starts.push({ client, configuration, params });
        await start(params);
        if (index < 3 || index === 3) {
          queueMicrotask(() => client.emit("exit", { code: 1, signal: null }));
        }
      };
      return client;
    }
  });

  try {
    await controller.start();
    await waitFor(() => messages.some((message) => message.endsWith("attempts exhausted.")));
    assert.equal(clients.length, 3, "initial startup plus every automatic retry should run");

    updateAuthority({ api, settings });
    trigger();
    await waitFor(() => clients.length === 5 && clients[4].status === "running" &&
      controller.restartPromise === undefined && controller.restartTimer === undefined);

    assert.equal(clients[3].retired, true, "the first explicit generation should fail before stability");
    assert.equal(clients[4].status, "running", "the bounded recovery should start a replacement");
    assertAuthority({ starts });

    return {
      scheduled: messages.filter((message) => message.includes("restart scheduled")),
      exhausted: messages.filter((message) => message.endsWith("attempts exhausted."))
    };
  } finally {
    await controller.dispose();
  }
}

function folder(name) {
  return {
    name,
    uri: { fsPath: `/workspace/${name}`, toString: () => `file:///workspace/${name}` }
  };
}
