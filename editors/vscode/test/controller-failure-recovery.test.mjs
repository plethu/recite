import test from "node:test";
import assert from "node:assert/strict";
import { ExtensionController } from "../src/controller.js";
import { ClientFailure, ClientFailureKind } from "../src/client-failure.js";
import { StartupOutcomeKind } from "../src/startup-outcome.js";
import {
  FakeClient,
  hostApi,
  output,
  waitFor
} from "./controller-fixtures.mjs";

test("server notifications preserve log visibility and project show-message severity", () => {
  const received = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const userInterface = {
    serverLogMessage: (message) => received.push(["log", message]),
    serverErrorMessage: (message) => received.push(["error", message]),
    serverWarningMessage: (message) => received.push(["warning", message]),
    serverInfoMessage: (message) => received.push(["info", message])
  };
  const controller = new ExtensionController(api, userInterface, { delete() {} });

  controller.handleNotification("window/logMessage", { type: 1, message: "diagnostic detail" });
  controller.handleNotification("window/showMessage", { type: 1, message: "failure" });
  controller.handleNotification("window/showMessage", { type: 2, message: "warning" });
  controller.handleNotification("window/showMessage", { type: 3, message: "information" });

  assert.deepEqual(received, [
    ["log", "diagnostic detail"],
    ["error", "failure"],
    ["warning", "warning"],
    ["info", "information"]
  ]);
});

test("typed client failures select one localized UI category at the controller edge", () => {
  const received = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const userInterface = {
    serverTransportFailure: (detail) => received.push(["transport", detail]),
    serverProtocolFailure: () => received.push(["protocol"]),
    serverLifecycleFailure: (detail) => received.push(["lifecycle", detail])
  };
  const controller = new ExtensionController(api, userInterface, { delete() {} });
  const transportClient = {};
  controller.client = transportClient;
  controller.handleClientFailure(
    transportClient,
    new ClientFailure(ClientFailureKind.Transport, "EPIPE")
  );
  const protocolClient = {};
  controller.client = protocolClient;
  controller.handleClientFailure(
    protocolClient,
    new ClientFailure(ClientFailureKind.Protocol)
  );
  const lifecycleClient = {};
  controller.client = lifecycleClient;
  controller.handleClientFailure(
    lifecycleClient,
    new ClientFailure(ClientFailureKind.Lifecycle, "code=1")
  );

  assert.deepEqual(received, [["transport", "EPIPE"], ["protocol"], ["lifecycle", "code=1"]]);
  assert.equal(transportClient.failureReported, true);
  assert.equal(protocolClient.failureReported, true);
  assert.equal(lifecycleClient.failureReported, true);
});

test("a terminal child failure schedules recovery without waiting for exit", async () => {
  const messages = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const clients = [];
  const controller = new ExtensionController(api, output(messages), { delete() {} }, {
    createClient: () => {
      const client = new FakeClient();
      clients.push(client);
      return client;
    }
  });

  await controller.start();
  clients[0].status = "stopped";
  clients[0].emit("failure", new ClientFailure(ClientFailureKind.Lifecycle, "child failed"));

  assert.deepEqual(messages, [
    "Recite language server lifecycle failed: child failed.",
    "Recite language server restart scheduled in 100 milliseconds."
  ]);
  assert.equal(controller.restartTimer !== undefined, true);
  await controller.dispose();
});

test("startup failure events are not reported a second time by start rejection", async () => {
  const received = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const userInterface = {
    serverProtocolFailure: () => received.push("protocol")
  };
  const controller = new ExtensionController(api, userInterface, { delete() {} }, {
    createClient: () => {
      const client = new FakeClient();
      client.start = async () => {
        const failure = new ClientFailure(ClientFailureKind.Protocol);
        client.emit("failure", failure);
        throw failure;
      };
      return client;
    }
  });

  assert.equal((await controller.start()).kind, StartupOutcomeKind.RetryableFailure);
  assert.deepEqual(received, ["protocol"]);
});

test("scheduled retry failures do not repeat visible lifecycle notifications", async () => {
  const received = [];
  let attempts = 0;
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const userInterface = {
    serverLifecycleFailure: (detail) => received.push(["failure", detail]),
    restartScheduled: (detail) => received.push(["scheduled", detail]),
    restartExhausted: () => received.push(["exhausted"]),
    serverLogMessage() {}
  };
  const controller = new ExtensionController(api, userInterface, { delete() {} }, {
    createClient: () => {
      attempts += 1;
      const client = new FakeClient();
      client.start = async () => {
        client.status = "stopped";
        throw new Error("ENOENT");
      };
      return client;
    }
  });

  controller.handleStartFailure(new Error("ENOENT"));
  controller.scheduleRestart();
  await waitFor(() => attempts === 1, 250);

  assert.deepEqual(received.slice(0, 2), [["failure", "ENOENT"], ["scheduled", 100]]);
  assert.equal(received.filter(([kind]) => kind === "failure").length, 1);
  await controller.dispose();
});

test("missing executable retries through the complete budget before one exhaustion notice", async () => {
  const received = [];
  let attempts = 0;
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const userInterface = {
    serverLifecycleFailure: (detail) => received.push(["failure", detail]),
    restartScheduled: (milliseconds) => received.push(["scheduled", milliseconds]),
    restartExhausted: () => received.push(["exhausted"])
  };
  const controller = new ExtensionController(api, userInterface, { delete() {} }, {
    restartDelaysMs: [0, 0, 0],
    createClient: () => {
      attempts += 1;
      const client = new FakeClient();
      client.start = async () => {
        client.status = "stopped";
        throw new Error("ENOENT");
      };
      return client;
    }
  });

  controller.scheduleRestart();
  await waitFor(() => received.some(([kind]) => kind === "exhausted"), 250);

  assert.equal(attempts, 3);
  assert.deepEqual(received, [["scheduled", 0], ["scheduled", 0], ["scheduled", 0], ["exhausted"]]);
  await controller.dispose();
});

test("activation turns an initial missing executable into a bounded retry sequence", async () => {
  const received = [];
  let attempts = 0;
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const userInterface = {
    serverLifecycleFailure: (detail) => received.push(["failure", detail]),
    restartScheduled: (milliseconds) => received.push(["scheduled", milliseconds]),
    restartExhausted: () => received.push(["exhausted"])
  };
  const controller = new ExtensionController(api, userInterface, { delete() {} }, {
    restartDelaysMs: [0, 0, 0],
    createClient: () => {
      attempts += 1;
      const client = new FakeClient();
      client.start = async () => {
        client.status = "stopped";
        throw new Error("ENOENT");
      };
      return client;
    }
  });

  const outcome = await controller.start();
  assert.equal(outcome.kind, StartupOutcomeKind.RetryableFailure);
  controller.handleStartOutcome(outcome);
  await waitFor(() => received.some(([kind]) => kind === "exhausted"), 250);

  assert.equal(attempts, 4, "initial startup plus every configured retry should be attempted");
  assert.deepEqual(received, [
    ["failure", "ENOENT"], ["scheduled", 0], ["scheduled", 0], ["scheduled", 0], ["exhausted"]
  ]);
  await controller.dispose();
});

test("retry-phase exit-only failures stay quiet through the budget", async () => {
  const received = [];
  const clients = [];
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const userInterface = {
    serverExited: () => received.push(["exited"]),
    restartScheduled: (milliseconds) => received.push(["scheduled", milliseconds]),
    restartExhausted: () => received.push(["exhausted"])
  };
  const controller = new ExtensionController(api, userInterface, { delete() {} }, {
    restartDelaysMs: [0, 0, 0],
    createClient: () => {
      const client = new FakeClient();
      clients.push(client);
      client.start = async () => {
        client.status = "running";
        queueMicrotask(() => client.emit("exit", { code: 1, signal: null }));
      };
      return client;
    }
  });

  const outcome = await controller.start();
  assert.equal(outcome.kind, StartupOutcomeKind.Started);
  await waitFor(() => received.some(([kind]) => kind === "exhausted"), 250);

  assert.equal(clients.length, 4, "initial exit plus every configured retry should be attempted");
  assert.deepEqual(received, [
    ["exited"], ["scheduled", 0], ["scheduled", 0], ["scheduled", 0], ["exhausted"]
  ]);
  await controller.dispose();
});

test("failure-emitting retries continue through the complete budget without failure spam", async () => {
  const received = [];
  let attempts = 0;
  const api = hostApi({ isTrusted: () => true, onDidGrantWorkspaceTrust: () => ({ dispose() {} }) });
  const userInterface = {
    serverLifecycleFailure: (detail) => received.push(["failure", detail]),
    restartScheduled: (milliseconds) => received.push(["scheduled", milliseconds]),
    restartExhausted: () => received.push(["exhausted"])
  };
  const controller = new ExtensionController(api, userInterface, { delete() {} }, {
    restartDelaysMs: [0, 0, 0],
    createClient: () => {
      attempts += 1;
      const client = new FakeClient();
      client.start = async () => {
        client.status = "stopped";
        const failure = new ClientFailure(ClientFailureKind.Lifecycle, "ENOENT");
        client.emit("failure", failure);
        throw failure;
      };
      return client;
    }
  });

  controller.scheduleRestart();
  await waitFor(() => received.some(([kind]) => kind === "exhausted"), 250);

  assert.equal(attempts, 3);
  assert.deepEqual(received, [["scheduled", 0], ["scheduled", 0], ["scheduled", 0], ["exhausted"]]);
  await controller.dispose();
});
