import * as vscode from "vscode";
import { ReciteLanguageClient } from "./lsp-client.js";
import { initializeParams, readConfiguration } from "./configuration.js";
import {
  lspCodeActionsToVscode,
  lspCompletionItems,
  lspDiagnosticToVscode,
  lspHoverToVscode,
  lspLocationToVscode,
  lspRangeToVscode,
  lspWorkspaceEditToVscode
} from "./lsp-features.js";

const SELECTOR = [
  { scheme: "file", language: "recite" },
  { scheme: "untitled", language: "recite" }
];
const DIAGNOSTICS_METHOD = "textDocument/publishDiagnostics";

let activeController;

export async function activate(context) {
  const output = vscode.window.createOutputChannel("Recite");
  const diagnostics = vscode.languages.createDiagnosticCollection("recite");
  const controller = new ExtensionController(vscode, output, diagnostics);
  activeController = controller;
  context.subscriptions.push(output, diagnostics, controller);
  await controller.start().catch((error) => {
    output.appendLine(`Unable to start recite-lsp: ${error.message}`);
    void vscode.window.showErrorMessage(
      vscode.l10n.t("Recite language server could not be started: {0}", error.message)
    );
  });
}

export async function deactivate() {
  const controller = activeController;
  activeController = undefined;
  await controller?.dispose();
}

class ExtensionController {
  constructor(api, output, diagnostics) {
    this.api = api;
    this.output = output;
    this.diagnostics = diagnostics;
    this.client = undefined;
    this.serverErrorShown = false;
    this.restartPromise = undefined;
    this.subscriptions = [];
    this.providersRegistered = false;
  }

  async start() {
    if (this.client) return;
    const configuration = readConfiguration(this.api);
    const client = new ReciteLanguageClient(configuration);
    this.client = client;
    client.on("notification", (method, params) => this.handleNotification(method, params));
    client.on("stderr", (message) => this.output.append(message));
    client.on("serverError", (error) => this.handleServerError(error));
    if (!this.providersRegistered) {
      this.registerDocumentLifecycle();
      this.registerFeatureProviders();
      this.providersRegistered = true;
    }
    await client.start(initializeParams(this.api, configuration.projectRoot));
    for (const document of this.api.workspace.textDocuments) this.open(document);
  }

  registerDocumentLifecycle() {
    this.subscriptions.push(
      this.api.workspace.onDidOpenTextDocument((document) => this.open(document)),
      this.api.workspace.onDidChangeTextDocument((event) => {
        if (!isReciteDocument(event.document)) return;
        this.client?.notify("textDocument/didChange", {
          textDocument: { uri: event.document.uri.toString(), version: event.document.version },
          contentChanges: [{ text: event.document.getText() }]
        });
      }),
      this.api.workspace.onDidSaveTextDocument((document) => {
        if (!isReciteDocument(document)) return;
        this.client?.notify("textDocument/didSave", {
          textDocument: { uri: document.uri.toString() }
        });
      }),
      this.api.workspace.onDidCloseTextDocument((document) => {
        if (!isReciteDocument(document)) return;
        const uri = document.uri.toString();
        this.client?.notify("textDocument/didClose", { textDocument: { uri } });
        this.diagnostics.delete(document.uri);
      }),
      this.api.workspace.onDidChangeConfiguration((event) => {
        if (!event.affectsConfiguration("recite.lsp")) return;
        void this.restart();
      })
    );
  }

  registerFeatureProviders() {
    const send = (method, params, token) => {
      const client = this.client;
      if (!client) return Promise.reject(new Error("recite-lsp is not running"));
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
      this.api.languages.registerReferenceProvider(
        SELECTOR,
        {
          provideReferences: (document, position, context, token) => request(
            "textDocument/references", document, position, token,
            { context: { includeDeclaration: context.includeDeclaration } }
          ).then((result) => locations(this.api, result))
        }
      ),
      this.api.languages.registerRenameProvider(SELECTOR, {
        prepareRename: (document, position, token) => request(
          "textDocument/prepareRename", document, position, token
        ).then((result) => result ? lspRangeToVscode(this.api, result.range ?? result) : undefined),
        provideRenameEdits: (document, position, newName, token) => request(
          "textDocument/rename", document, position, token, { newName }
        ).then((result) => lspWorkspaceEditToVscode(this.api, result, getDocument))
      }),
      this.api.languages.registerCodeActionsProvider(SELECTOR, {
        provideCodeActions: (document, range, context, token) => send("textDocument/codeAction", {
            textDocument: { uri: document.uri.toString() },
            range: {
              start: { line: range.start.line, character: range.start.character },
              end: { line: range.end.line, character: range.end.character }
            },
            context: { diagnostics: context.diagnostics.map(toLspDiagnostic) }
          }, token).then((result) => lspCodeActionsToVscode(this.api, result, getDocument))
      })
    );
  }

  open(document) {
    if (!isReciteDocument(document) || !this.client || this.client.status === "stopped") return;
    this.client.notify("textDocument/didOpen", {
      textDocument: {
        uri: document.uri.toString(),
        languageId: "recite",
        version: document.version,
        text: document.getText()
      }
    });
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
    this.output.appendLine(`recite-lsp: ${error.message}`);
    if (this.serverErrorShown) return;
    this.serverErrorShown = true;
    void this.api.window.showErrorMessage(
      this.api.l10n.t("Recite language server stopped: {0}", error.message)
    );
  }

  async restart() {
    if (this.restartPromise) return this.restartPromise;
    this.restartPromise = (async () => {
      await this.client?.stop();
      this.client = undefined;
      this.serverErrorShown = false;
      await this.start();
    })().finally(() => { this.restartPromise = undefined; });
    return this.restartPromise;
  }

  async dispose() {
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

function toLspDiagnostic(diagnostic) {
  return {
    range: {
      start: { line: diagnostic.range.start.line, character: diagnostic.range.start.character },
      end: { line: diagnostic.range.end.line, character: diagnostic.range.end.character }
    },
    severity: diagnostic.severity,
    code: diagnostic.code,
    source: diagnostic.source,
    message: diagnostic.message
  };
}
