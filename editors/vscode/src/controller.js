import { ReciteLanguageClient } from "./lsp-client.js";
import { initializeParams, readConfiguration } from "./configuration.js";
import {
  lspDiagnosticToVscode,
  workspaceEditIsCurrent,
} from "./lsp-features.js";
import { clientMessage } from "./messages.js";
import { registerDocumentLifecycle } from "./document-lifecycle.js";
import { registerFeatureProviders } from "./providers.js";
import { WatcherRegistry } from "./watchers.js";

const DIAGNOSTICS_METHOD = "textDocument/publishDiagnostics";
const APPLY_CODE_ACTION_COMMAND = "recite.applyCodeAction";
const RESTART_DELAYS_MS = [100, 500, 1_000, 2_000, 5_000];
const STABLE_RUN_MS = 10_000;

export class ExtensionController {
  constructor(api, output, diagnostics, options = {}) {
    this.api = api;
    this.output = output;
    this.diagnostics = diagnostics;
    this.createClient = options.createClient ?? ((configuration, callbacks) =>
      new ReciteLanguageClient({ ...configuration, ...callbacks }));
    this.stableRunMs = options.stableRunMs ?? STABLE_RUN_MS;
    this.client = undefined;
    this.restartPromise = undefined;
    this.restartTimer = undefined;
    this.stableRunTimer = undefined;
    this.restartAttempt = 0;
    this.stopping = false;
    this.disposed = false;
    this.subscriptions = [];
    this.providersRegistered = false;
    this.trustListenerRegistered = false;
    this.documents = new Map();
    this.editCommands = new Map();
    this.nextEditCommandId = 1;
    this.editCommandRegistered = false;
    this.watchers = new WatcherRegistry(this);
  }

  async start() {
    this.ensureSubscriptions();
    if (this.disposed || this.api.workspace.isTrusted === false) return false;
    if (this.client && this.client.status !== "stopped") return true;
    this.client = undefined;
    const configuration = readConfiguration(this.api);
    const client = this.createClient(configuration, {
      onRegisterCapability: (params) => this.registerCapabilities(params),
      onUnregisterCapability: (params) => this.unregisterCapabilities(params)
    });
    this.client = client;
    client.on("notification", (method, params) => this.handleNotification(method, params));
    client.on("stderr", (message) => this.output.append(message));
    client.on("serverError", (error) => this.handleServerError(error));
    client.on("exit", (event) => this.handleExit(client, event));
    try {
      await client.start(initializeParams(
        this.api,
        configuration.projectRoot,
        configuration.projectRootOverridden
      ));
      this.replayOpenDocuments();
      this.watchers.flush();
      this.scheduleStableReset(client);
      return true;
    } catch (error) {
      if (this.client === client) this.client = undefined;
      throw error;
    }
  }

  ensureSubscriptions() {
    this.ensureTrustListener();
    if (this.providersRegistered) return;
    registerDocumentLifecycle(this);
    registerFeatureProviders(this);
    this.registerEditCommand();
    this.providersRegistered = true;
  }

  ensureTrustListener() {
    if (this.trustListenerRegistered || !this.api.workspace.onDidGrantWorkspaceTrust) return;
    this.trustListenerRegistered = true;
    this.subscriptions.push(this.api.workspace.onDidGrantWorkspaceTrust(() => {
      void this.start().catch((error) => this.handleStartFailure(error));
    }));
  }

  registerEditCommand() {
    if (this.editCommandRegistered || !this.api.commands?.registerCommand) return;
    this.editCommandRegistered = true;
    this.subscriptions.push(this.api.commands.registerCommand(
      APPLY_CODE_ACTION_COMMAND,
      (id) => this.applyEditCommand(id)
    ));
  }

  createEditCommand(title, edit) {
    if (!this.editCommandRegistered) return undefined;
    const id = String(this.nextEditCommandId++);
    this.editCommands.set(id, edit);
    if (this.api.Command) return new this.api.Command(title, APPLY_CODE_ACTION_COMMAND, id);
    return { title, command: APPLY_CODE_ACTION_COMMAND, arguments: [id] };
  }

  applyEditCommand(id) {
    const edit = this.editCommands.get(id);
    this.editCommands.delete(id);
    if (!edit || !workspaceEditIsCurrent(edit)) return false;
    return this.api.workspace.applyEdit(edit);
  }

  handleStartFailure(error) {
    this.output.appendLine(clientMessage(this.api, "lsp-client-start-failed", error.message));
  }

  handleExit(client, event) {
    if (this.client !== client || this.stopping || this.disposed) return;
    this.clearStableReset();
    this.client = undefined;
    this.output.appendLine(clientMessage(this.api, "lsp-client-exited", event.code ?? "unknown"));
    this.scheduleRestart();
  }

  scheduleRestart() {
    if (this.restartTimer || this.disposed || this.stopping || this.api.workspace.isTrusted === false) return;
    if (this.restartAttempt >= RESTART_DELAYS_MS.length) {
      this.output.appendLine(clientMessage(this.api, "lsp-client-restart-exhausted", "restart attempts exhausted"));
      return;
    }
    const delay = RESTART_DELAYS_MS[this.restartAttempt++];
    this.output.appendLine(clientMessage(this.api, "lsp-client-restart-scheduled", `${delay} ms`));
    this.restartTimer = setTimeout(() => {
      this.restartTimer = undefined;
      void this.start().catch((error) => {
        this.handleStartFailure(error);
        this.scheduleRestart();
      });
    }, delay);
    this.restartTimer.unref?.();
  }

  scheduleStableReset(client) {
    this.clearStableReset();
    const timer = setTimeout(() => {
      if (this.client === client && client.status === "running") {
        this.restartAttempt = 0;
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
    if (method === "window/logMessage" || method === "window/showMessage") {
      if (params?.message) this.output.appendLine(params.message);
    }
  }

  handleServerError(error) {
    this.output.appendLine(clientMessage(this.api, "lsp-client-error", error.message));
  }

  registerCapabilities(params) {
    this.watchers.registerCapabilities(params);
  }

  unregisterCapabilities(params) {
    this.watchers.unregisterCapabilities(params);
  }

  async restart() {
    if (this.restartPromise) return this.restartPromise;
    this.restartPromise = (async () => {
      this.stopping = true;
      this.clearStableReset();
      await this.client?.stop();
      this.client = undefined;
      this.stopping = false;
      await this.start();
    })().finally(() => { this.restartPromise = undefined; });
    return this.restartPromise;
  }

  async dispose() {
    this.disposed = true;
    this.stopping = true;
    if (this.restartTimer) clearTimeout(this.restartTimer);
    this.restartTimer = undefined;
    this.clearStableReset();
    this.watchers.dispose();
    this.editCommands.clear();
    for (const subscription of this.subscriptions.splice(0)) subscription.dispose();
    await this.client?.stop();
    this.client = undefined;
  }
}
