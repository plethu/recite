import { ReciteLanguageClient } from "./lsp-client.js";
import { initializeParams, readConfiguration } from "./configuration.js";
import {
  lspCodeActionsToVscode,
  lspCompletionItems,
  lspDiagnosticToVscode,
  lspHoverToVscode,
  lspLocationToVscode,
  lspRangeToVscode,
  lspWorkspaceEditToVscode,
  workspaceEditIsCurrent,
  vscodeDiagnosticToLsp,
} from "./lsp-features.js";
import { clientMessage } from "./messages.js";

const SELECTOR = [
  { scheme: "file", language: "recite" },
  { scheme: "untitled", language: "recite" }
];
const DIAGNOSTICS_METHOD = "textDocument/publishDiagnostics";
const WATCH_METHOD = "workspace/didChangeWatchedFiles";
const RESTART_DELAYS_MS = [100, 500, 1_000, 2_000, 5_000];

export class ExtensionController {
  constructor(api, output, diagnostics, options = {}) {
    this.api = api;
    this.output = output;
    this.diagnostics = diagnostics;
    this.createClient = options.createClient ?? ((configuration, callbacks) =>
      new ReciteLanguageClient({ ...configuration, ...callbacks }));
    this.client = undefined;
    this.restartPromise = undefined;
    this.restartTimer = undefined;
    this.restartAttempt = 0;
    this.stopping = false;
    this.disposed = false;
    this.subscriptions = [];
    this.providersRegistered = false;
    this.trustListenerRegistered = false;
    this.documents = new Map();
    this.watcherRegistrations = new Map();
    this.pendingWatchEvents = new Map();
    this.watchFlushTimer = undefined;
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
      this.flushWatchEvents();
      return true;
    } catch (error) {
      if (this.client === client) this.client = undefined;
      throw error;
    }
  }

  ensureSubscriptions() {
    this.ensureTrustListener();
    if (this.providersRegistered) return;
    this.registerDocumentLifecycle();
    this.registerFeatureProviders();
    this.providersRegistered = true;
  }

  ensureTrustListener() {
    if (this.trustListenerRegistered || !this.api.workspace.onDidGrantWorkspaceTrust) return;
    this.trustListenerRegistered = true;
    this.subscriptions.push(this.api.workspace.onDidGrantWorkspaceTrust(() => {
      void this.start().catch((error) => this.handleStartFailure(error));
    }));
  }

  handleStartFailure(error) {
    this.output.appendLine(clientMessage(this.api, "lsp-client-start-failed", error.message));
  }

  handleExit(client, event) {
    if (this.client !== client || this.stopping || this.disposed) return;
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

  registerDocumentLifecycle() {
    this.subscriptions.push(
      this.api.workspace.onDidOpenTextDocument((document) => this.open(document)),
      this.api.workspace.onDidChangeTextDocument((event) => {
        if (!isReciteDocument(event.document)) return;
        this.documents.set(event.document.uri.toString(), event.document);
        if (this.client?.status !== "running") return;
        this.client.notify("textDocument/didChange", {
          textDocument: { uri: event.document.uri.toString(), version: event.document.version },
          contentChanges: [{ text: event.document.getText() }]
        });
      }),
      this.api.workspace.onDidSaveTextDocument((document) => {
        if (!isReciteDocument(document)) return;
        if (this.client?.status !== "running") return;
        this.client.notify("textDocument/didSave", {
          textDocument: { uri: document.uri.toString() }
        });
      }),
      this.api.workspace.onDidCloseTextDocument((document) => {
        if (!isReciteDocument(document)) return;
        const uri = document.uri.toString();
        this.documents.delete(uri);
        if (this.client?.status === "running") {
          this.client.notify("textDocument/didClose", { textDocument: { uri } });
        }
        this.diagnostics.delete(document.uri);
      }),
      this.api.workspace.onDidChangeConfiguration((event) => {
        if (!event.affectsConfiguration("recite.lsp")) return;
        void this.restart().catch((error) => this.handleStartFailure(error));
      })
    );
  }

  registerFeatureProviders() {
    const send = (method, params, token) => {
      const client = this.client;
      if (!client || client.status !== "running") {
        return Promise.reject(new Error("recite-lsp is not running"));
      }
      return client.request(method, params)
        .then((result) => token?.isCancellationRequested ? undefined : result);
    };
    const request = (method, document, position, token, extra = {}) => send(method, {
        textDocument: { uri: document.uri.toString() },
        position: { line: position.line, character: position.character },
        ...extra
      }, token);
    const getDocument = (uri) => this.api.workspace.textDocuments.find(
      (document) => document.uri.toString() === uri.toString()
    );
    const provider = (callback, method, transform) => ({
      [callback]: (document, position, token, extra) => request(method, document, position, token, extra)
        .then((result) => transform(result))
    });

    this.subscriptions.push(
      this.api.languages.registerCompletionItemProvider(
        SELECTOR,
        provider("provideCompletionItems", "textDocument/completion", (result) => lspCompletionItems(this.api, result)),
        "(", "="
      ),
      this.api.languages.registerHoverProvider(
        SELECTOR,
        provider("provideHover", "textDocument/hover", (result) => lspHoverToVscode(this.api, result))
      ),
      this.api.languages.registerDefinitionProvider(
        SELECTOR,
        provider("provideDefinition", "textDocument/definition", (result) => locations(this.api, result))
      ),
      this.api.languages.registerReferenceProvider(SELECTOR, {
        provideReferences: (document, position, context, token) => request(
          "textDocument/references", document, position, token,
          { context: { includeDeclaration: context.includeDeclaration } }
        ).then((result) => locations(this.api, result))
      }),
      this.api.languages.registerRenameProvider(SELECTOR, {
        prepareRename: (document, position, token) => request(
          "textDocument/prepareRename", document, position, token
        ).then((result) => result ? lspRangeToVscode(this.api, result.range ?? result) : undefined),
        provideRenameEdits: (document, position, newName, token) => request(
          "textDocument/rename", document, position, token, { newName }
        ).then((result) => {
          const edit = lspWorkspaceEditToVscode(this.api, result, getDocument);
          return workspaceEditIsCurrent(edit) ? edit : undefined;
        })
      }),
      this.api.languages.registerCodeActionsProvider(SELECTOR, {
        provideCodeActions: (document, range, context, token) => send("textDocument/codeAction", {
            textDocument: { uri: document.uri.toString() },
            range: {
              start: { line: range.start.line, character: range.start.character },
              end: { line: range.end.line, character: range.end.character }
            },
            context: { diagnostics: context.diagnostics.map((diagnostic) =>
              vscodeDiagnosticToLsp(this.api, diagnostic)
            ) }
          }, token).then((result) => lspCodeActionsToVscode(this.api, result, getDocument))
      })
    );
  }

  open(document) {
    if (!isReciteDocument(document)) return;
    this.documents.set(document.uri.toString(), document);
    if (this.client?.status !== "running") return;
    this.sendOpen(document);
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

  async restart() {
    if (this.restartPromise) return this.restartPromise;
    this.restartPromise = (async () => {
      this.stopping = true;
      await this.client?.stop();
      this.client = undefined;
      this.stopping = false;
      this.restartAttempt = 0;
      await this.start();
    })().finally(() => { this.restartPromise = undefined; });
    return this.restartPromise;
  }

  async registerCapabilities(params) {
    for (const registration of params?.registrations ?? []) {
      if (registration.method !== WATCH_METHOD) continue;
      this.unregisterCapabilities({ unregistrations: [{ id: registration.id }] });
      this.installWatchRegistration(registration);
    }
  }

  unregisterCapabilities(params) {
    for (const registration of params?.unregisterations ?? params?.unregistrations ?? []) {
      const installed = this.watcherRegistrations.get(registration.id);
      if (!installed) continue;
      for (const disposable of installed) disposable.dispose();
      this.watcherRegistrations.delete(registration.id);
    }
  }

  installWatchRegistration(registration) {
    const installed = [];
    for (const watcher of registration.registerOptions?.watchers ?? []) {
      const fileWatcher = this.api.workspace.createFileSystemWatcher(this.watchPattern(watcher.globPattern));
      const kind = watcher.kind ?? 7;
      if (kind & 1) installed.push(fileWatcher.onDidCreate((uri) => this.queueWatchEvent(1, uri)));
      if (kind & 2) installed.push(fileWatcher.onDidChange((uri) => this.queueWatchEvent(2, uri)));
      if (kind & 4) installed.push(fileWatcher.onDidDelete((uri) => this.queueWatchEvent(3, uri)));
      installed.push(fileWatcher);
    }
    this.watcherRegistrations.set(registration.id, installed);
  }

  watchPattern(pattern) {
    if (typeof pattern === "string") return pattern;
    if (pattern?.baseUri && this.api.RelativePattern) {
      return new this.api.RelativePattern(this.api.Uri.parse(pattern.baseUri), pattern.pattern);
    }
    return pattern?.pattern ?? "**/*";
  }

  queueWatchEvent(type, uri) {
    const key = `${type}\0${uri.toString()}`;
    this.pendingWatchEvents.set(key, { type, uri: uri.toString() });
    if (this.watchFlushTimer) return;
    this.watchFlushTimer = setTimeout(() => {
      this.watchFlushTimer = undefined;
      this.flushWatchEvents();
    }, 0);
    this.watchFlushTimer.unref?.();
  }

  flushWatchEvents() {
    if (!this.pendingWatchEvents.size) return;
    if (!this.client || this.client.status === "stopped") return;
    const changes = [...this.pendingWatchEvents.values()]
      .sort((left, right) => left.type - right.type || left.uri.localeCompare(right.uri))
      .map(({ type, uri }) => ({ type, uri }));
    this.pendingWatchEvents.clear();
    this.client.notify(WATCH_METHOD, { changes });
  }

  async dispose() {
    this.disposed = true;
    this.stopping = true;
    if (this.restartTimer) clearTimeout(this.restartTimer);
    this.restartTimer = undefined;
    if (this.watchFlushTimer) clearTimeout(this.watchFlushTimer);
    this.watchFlushTimer = undefined;
    for (const installed of this.watcherRegistrations.values()) {
      for (const disposable of installed) disposable.dispose();
    }
    this.watcherRegistrations.clear();
    for (const subscription of this.subscriptions.splice(0)) subscription.dispose();
    await this.client?.stop();
    this.client = undefined;
  }
}

function isReciteDocument(document) {
  return document.languageId === "recite";
}

function locations(api, result) {
  const values = Array.isArray(result) ? result : result ? [result] : [];
  return values.map((location) => lspLocationToVscode(api, location)).filter(Boolean);
}
