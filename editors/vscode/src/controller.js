import { ReciteLanguageClient } from "./lsp-client.js";
import { initializeParams, readConfiguration } from "./configuration.js";
import { lspDiagnosticToVscode } from "./lsp-features.js";
import { registerDocumentLifecycle } from "./document-lifecycle.js";
import { registerFeatureProviders } from "./providers.js";
import { WatcherRegistry } from "./watchers.js";
import { EditCommandRegistry } from "./edit-commands.js";
import { asClientFailure, ClientFailureKind, isClientFailure } from "./client-failure.js";
import { RestartPolicy } from "./restart-policy.js";
import { StartupOutcomeKind, startupOutcome } from "./startup-outcome.js";

const DIAGNOSTICS_METHOD = "textDocument/publishDiagnostics";
const STABLE_RUN_MS = 10_000;

export class ExtensionController {
  constructor(api, userInterface, diagnostics, options = {}) {
    this.api = api;
    this.userInterface = userInterface;
    this.diagnostics = diagnostics;
    this.createClient = options.createClient ?? ((configuration, callbacks) =>
      new ReciteLanguageClient({ ...configuration, ...callbacks }));
    this.stableRunMs = options.stableRunMs ?? STABLE_RUN_MS;
    this.restartPolicy = new RestartPolicy(options.restartDelaysMs);
    this.client = undefined;
    this.restartPromise = undefined;
    this.restartRevision = 0;
    this.restartTimer = undefined;
    this.stableRunTimer = undefined;
    this.stopping = false;
    this.disposed = false;
    this.subscriptions = [];
    this.providersRegistered = false;
    this.trustListenerRegistered = false;
    this.documents = new Map();
    this.editCommands = new EditCommandRegistry(this.api, this.userInterface, options);
    this.watchers = new WatcherRegistry(this);
  }

  async start(phase = "initial") {
    this.ensureSubscriptions();
    if (this.disposed || this.api.workspace.isTrusted === false) {
      return startupOutcome(StartupOutcomeKind.Refused);
    }
    if (this.client && this.client.status !== "stopped") {
      return startupOutcome(StartupOutcomeKind.Started);
    }
    if (this.client?.status === "stopped") {
      // A child error may be terminal without ever producing an exit event.
      // Finish that client's local transport cleanup before dropping the
      // controller reference during the bounded retry.
      try { await this.client.stop?.(); } catch { /* already torn down */ }
    }
    this.client = undefined;
    let configuration;
    try {
      configuration = readConfiguration(this.api, this.userInterface);
    } catch (error) {
      return startupOutcome(StartupOutcomeKind.Refused, error);
    }
    let client;
    try {
      client = this.createClient(configuration, {
        onRegisterCapability: (params) => this.registerCapabilities(params),
        onUnregisterCapability: (params) => this.unregisterCapabilities(params)
      });
    } catch (error) {
      return startupOutcome(
        StartupOutcomeKind.RetryableFailure,
        asClientFailure(ClientFailureKind.Lifecycle, error)
      );
    }
    this.client = client;
    client.on("notification", (method, params) => this.handleNotification(method, params));
    client.on("stderr", (message) => this.userInterface.serverStderr(message));
    client.on("failure", (failure) => this.handleClientFailure(
      client, failure, { notify: phase !== "retrying" }
    ));
    client.on("exit", (event) => this.handleExit(client, event, phase));
    try {
      await client.start(initializeParams(
        this.api,
        configuration.projectRoot,
        configuration.projectRootOverridden
      ));
      this.replayOpenDocuments();
      this.watchers.flush();
      this.scheduleStableReset(client);
      return startupOutcome(StartupOutcomeKind.Started);
    } catch (error) {
      if (this.client === client) this.client = undefined;
      // Transport/protocol failures have already been projected by the
      // client's failure event. Exit handling likewise owns its notification;
      // do not turn either one into a duplicate generic start failure.
      return startupOutcome(
        StartupOutcomeKind.RetryableFailure,
        error,
        client.failureReported || client.exitReported
      );
    }
  }

  ensureSubscriptions() {
    this.ensureTrustListener();
    if (this.providersRegistered) return;
    registerDocumentLifecycle(this);
    registerFeatureProviders(this);
    this.editCommands.register(this.subscriptions);
    this.providersRegistered = true;
  }

  ensureTrustListener() {
    if (this.trustListenerRegistered || !this.api.workspace.onDidGrantWorkspaceTrust) return;
    this.trustListenerRegistered = true;
    this.subscriptions.push(this.api.workspace.onDidGrantWorkspaceTrust(() => {
      void this.start()
        .then((outcome) => this.handleStartOutcome(outcome))
        .catch((error) => this.handleUnexpectedStartFailure(error));
    }));
  }

  createEditCommand(title, edit, batch) {
    return this.editCommands.create(title, edit, batch);
  }

  createEditCommandBatch() {
    return this.editCommands.beginBatch();
  }

  discardEditCommandsForDocument(document, reason = "document-stale") {
    this.editCommands.discardForDocument(document, reason);
  }

  handleStartFailure(error) {
    const failure = isClientFailure(error)
      ? error
      : asClientFailure(ClientFailureKind.Lifecycle, error);
    this.handleClientFailure(this.client, failure);
  }

  handleStartOutcome(outcome, phase = "initial") {
    if (outcome.kind === StartupOutcomeKind.Refused) {
      if (outcome.error) this.handleStartFailure(outcome.error);
      return outcome;
    }
    if (outcome.kind === StartupOutcomeKind.RetryableFailure) {
      if (phase !== "retrying" && !outcome.reported && outcome.error) {
        this.handleStartFailure(outcome.error);
      }
      this.scheduleRestart();
    }
    return outcome;
  }

  handleUnexpectedStartFailure(error) {
    this.handleStartFailure(error);
    this.scheduleRestart();
  }

  handleExit(client, event, phase = "initial") {
    if (this.client !== client || this.stopping || this.disposed) return;
    this.clearStableReset();
    this.client = undefined;
    client.exitReported = true;
    if (phase !== "retrying" && !client.failureReported) this.userInterface.serverExited();
    this.scheduleRestart();
  }

  scheduleRestart() {
    if (this.restartTimer || this.disposed || this.stopping || this.api.workspace.isTrusted === false) return;
    const delay = this.restartPolicy.nextDelay();
    if (delay === undefined) {
      if (this.restartPolicy.reportExhausted()) this.userInterface.restartExhausted();
      return;
    }
    this.userInterface.restartScheduled(delay);
    this.restartTimer = setTimeout(() => {
      this.restartTimer = undefined;
      // A scheduled retry is already represented by the output-channel
      // schedule message. Keep transient failures quiet; the exhausted
      // budget below is the visible terminal notification.
      void this.start("retrying")
        .then((outcome) => this.handleStartOutcome(outcome, "retrying"))
        .catch((error) => this.handleUnexpectedRetryFailure(error));
    }, delay);
    this.restartTimer.unref?.();
  }

  handleUnexpectedRetryFailure(error) {
    this.scheduleRestart();
  }

  scheduleStableReset(client) {
    this.clearStableReset();
    const timer = setTimeout(() => {
      if (this.client === client && client.status === "running") {
        this.restartPolicy.reset();
        this.stableRunTimer = undefined;
      }
    }, this.stableRunMs);
    timer.unref?.();
    this.stableRunTimer = timer;
  }

  clearStableReset() {
    if (this.stableRunTimer) clearTimeout(this.stableRunTimer);
    this.stableRunTimer = undefined;
  }

  open(document) {
    if (document.languageId !== "recite") return;
    this.documents.set(document.uri.toString(), document);
    if (this.client?.status === "running") this.sendOpen(document);
  }

  sendOpen(document) {
    this.client?.notify("textDocument/didOpen", {
      textDocument: {
        uri: document.uri.toString(),
        languageId: "recite",
        version: document.version,
        text: document.getText()
      }
    });
  }

  replayOpenDocuments() {
    for (const document of [...this.documents.values()].sort((left, right) =>
      left.uri.toString().localeCompare(right.uri.toString())
    )) this.sendOpen(document);
  }

  handleNotification(method, params) {
    if (method === DIAGNOSTICS_METHOD) {
      const uri = this.api.Uri.parse(params.uri);
      const document = this.api.workspace.textDocuments.find(
        (candidate) => candidate.uri.toString() === uri.toString()
      );
      if (params.version !== undefined && document && params.version !== document.version) return;
      this.diagnostics.set(
        uri,
        (params.diagnostics ?? []).map((diagnostic) => lspDiagnosticToVscode(this.api, diagnostic))
      );
      return;
    }
    if (method === "window/logMessage") {
      if (params?.message) this.userInterface.serverLogMessage(params.message);
      return;
    }
    if (method === "window/showMessage") {
      if (!params?.message) return;
      switch (params.type) {
        case 1: this.userInterface.serverErrorMessage(params.message); break;
        case 2: this.userInterface.serverWarningMessage(params.message); break;
        default: this.userInterface.serverInfoMessage(params.message); break;
      }
    }
  }

  handleClientFailure(client, failure, { notify = true } = {}) {
    if (client && this.client !== client) return;
    if (client) client.failureReported = true;
    if (notify) {
      switch (failure.kind) {
        case ClientFailureKind.Transport:
          this.userInterface.serverTransportFailure(failure.detail);
          break;
        case ClientFailureKind.Protocol:
          this.userInterface.serverProtocolFailure();
          break;
        case ClientFailureKind.Lifecycle:
        default:
          this.userInterface.serverLifecycleFailure(failure.detail);
          break;
      }
    }
    // A child error is terminal even when the process never emits `exit`.
    // The client moves to stopped before emitting failure, so this path owns
    // recovery for that otherwise silent lifecycle ending. Exit events remain
    // idempotent through the restart timer guard and failureReported marker.
    if (client && this.client === client && client.status === "stopped") {
      this.clearStableReset();
      this.scheduleRestart();
    }
  }

  registerCapabilities(params) {
    this.watchers.registerCapabilities(params);
  }

  unregisterCapabilities(params) {
    this.watchers.unregisterCapabilities(params);
  }

  async restart() {
    this.restartRevision += 1;
    if (this.restartPromise) return this.restartPromise;
    this.restartPromise = (async () => {
      let restartRevision;
      do {
        this.stopping = true;
        this.clearStableReset();
        await this.client?.stop();
        this.client = undefined;
        this.stopping = false;
        restartRevision = this.restartRevision;
        const outcome = await this.start();
        this.handleStartOutcome(outcome);
        // A failed start already owns a bounded retry schedule. A queued
        // configuration change will be read by that retry; do not start a
        // second recovery path outside the restart budget.
        if (outcome.kind !== StartupOutcomeKind.Started) break;
        // Capture the revision after stopping the old client. Changes made
        // while it was stopping are included by this start; only changes
        // after that snapshot need a follow-up restart.
      } while (this.restartRevision !== restartRevision && !this.disposed);
    })().finally(() => {
      this.restartPromise = undefined;
    });
    return this.restartPromise;
  }

  async dispose() {
    this.disposed = true;
    this.stopping = true;
    if (this.restartTimer) clearTimeout(this.restartTimer);
    this.restartTimer = undefined;
    this.clearStableReset();
    this.watchers.dispose();
    this.editCommands.dispose();
    for (const subscription of this.subscriptions.splice(0)) subscription.dispose();
    await this.client?.stop();
    this.client = undefined;
  }
}
