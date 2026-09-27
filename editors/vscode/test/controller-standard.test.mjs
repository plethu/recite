import test from "node:test";
import assert from "node:assert/strict";
import { ExtensionController } from "../src/controller.js";
import { FakeClient, hostApi, output } from "./controller-fixtures.mjs";

test("untrusted workspaces do not start a language server or command process", async () => {
  const api = hostApi({ isTrusted: () => false });
  let created = 0;
  const controller = new ExtensionController(api, output(), undefined, {
    createClient: () => { created++; return new FakeClient(); }
  });
  const outcome = await controller.start();
  assert.equal(outcome.kind, "refused");
  assert.equal(created, 0);
  assert.equal(api.registeredProviders.length, 0);
  await controller.dispose();
});

test("restart retires the prior client and shutdown cleans the active client", async () => {
  const api = hostApi({ isTrusted: () => true });
  const clients = [];
  const controller = new ExtensionController(api, output(), undefined, {
    createClient: () => {
      const client = new FakeClient();
      clients.push(client);
      return client;
    },
    restartDelaysMs: []
  });
  assert.equal((await controller.start()).kind, "started");
  assert.equal(api.registeredProviders.length, 0);
  await controller.restart();
  assert.equal(clients.length, 2);
  assert.equal(clients[0].status, "stopped");
  assert.equal(clients[1].status, "running");
  await controller.dispose();
  assert.equal(clients[1].status, "stopped");
});
