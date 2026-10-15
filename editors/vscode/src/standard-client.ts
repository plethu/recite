import { EventEmitter } from "node:events";
import { lspCodeActionsToVscode, vscodeCodeActionContextToLsp } from "./lsp-features.ts";
import type { HostApi, ServerConfiguration, ClientStatus } from "./types.ts";
import type { ExtensionController } from "./controller.ts";
import type { LanguageClientOptions } from "vscode-languageclient/node.js";
import type { TextDocument, Range, CodeActionContext, CancellationToken, Disposable } from "vscode";

type ClientModule = typeof import("vscode-languageclient/node.js");

// The languageclient owns LSP framing, synchronization, diagnostics, dynamic
// registration, and standard providers. This adapter retains only Recite's
// guarded edit and lifecycle policy.
export class ReciteStandardClient extends EventEmitter {
  declare api: HostApi;
  declare configuration: ServerConfiguration;
  declare controller: ExtensionController;
  declare load: () => Promise<ClientModule & { default?: ClientModule }>;
  declare status: ClientStatus;
  declare retired: boolean;
  declare client: import("vscode-languageclient/node.js").LanguageClient;
  declare stateListener: Disposable | undefined;

  constructor(
    api: HostApi,
    configuration: ServerConfiguration,
    controller: ExtensionController,
    load: () => Promise<ClientModule & { default?: ClientModule }> = () =>
      import("vscode-languageclient/node.js"),
  ) {
    super();
    this.api = api;
    this.configuration = configuration;
    this.controller = controller;
    this.load = load;
    this.status = "idle";
    this.retired = false;
  }

  async start() {
    const loaded = await this.load();
    const {
      LanguageClient,
      State,
      ErrorAction,
      CloseAction,
      CancellationToken: ProtocolToken,
    } = loaded.default ?? loaded;
    const { command, args, cwd } = this.configuration;
    const server = { command, args, options: { cwd: cwd!, shell: false } };
    const options: LanguageClientOptions = {
      documentSelector: [
        { scheme: "file", language: "recite" },
        { scheme: "untitled", language: "recite" },
      ],
      diagnosticCollectionName: "recite",
      workspaceFolder:
        this.configuration.projectRootOverridden && cwd
          ? {
              uri: this.api.Uri.file(cwd),
              name: cwd.split(/[\\/]/).at(-1)!,
              index: 0,
            }
          : undefined!,
      middleware: {
        // The host's native edit application does not preserve LSP document
        // versions. Keep code actions and rename on Recite's guarded commands.
        provideCodeActions: (document, range, context, token) =>
          this.guardedCodeActions(document, range, context, token),
        handleRegisterCapability: async (params, next) => {
          const result = await next(
            {
              ...params,
              registrations:
                params.registrations?.filter((entry) => entry.method !== "textDocument/rename") ??
                [],
            },
            ProtocolToken.None,
          );
          if (result !== undefined) throw result;
        },
        provideRenameEdits: () => undefined,
        prepareRename: () => undefined,
      },
      errorHandler: {
        error: (error) => {
          this.emit("failure", error);
          return { action: ErrorAction.Shutdown };
        },
        closed: () => {
          this.status = "stopped";
          this.emit("exit", {});
          return { action: CloseAction.DoNotRestart };
        },
      },
    };
    this.client = new LanguageClient("recite", "Recite", server, options);
    this.stateListener = this.client.onDidChangeState(({ newState }) => {
      if (newState === State.Stopped && this.status !== "stopped") {
        this.status = "stopped";
        this.emit("exit", {});
      }
    });
    this.status = "starting";
    try {
      await this.client.start();
      this.client.getFeature?.("textDocument/rename")?.clear();
      this.status = "running";
    } catch (error) {
      this.status = "stopped";
      throw error;
    }
  }

  request(method: string, params: unknown, token?: CancellationToken): Promise<unknown> {
    return token === undefined
      ? this.client.sendRequest(method, params)
      : this.client.sendRequest(method, params, token);
  }

  async guardedCodeActions(
    document: TextDocument,
    range: Range,
    context: CodeActionContext,
    token: CancellationToken,
  ) {
    const result = await this.request(
      "textDocument/codeAction",
      {
        textDocument: { uri: document.uri.toString() },
        range: {
          start: { line: range.start.line, character: range.start.character },
          end: { line: range.end.line, character: range.end.character },
        },
        context: vscodeCodeActionContextToLsp(this.api, context),
      },
      token,
    );
    if (token?.isCancellationRequested) return undefined;
    const batch = this.controller.createEditCommandBatch();
    try {
      return lspCodeActionsToVscode(
        this.api,
        result,
        (uri) =>
          this.api.workspace.textDocuments.find(
            (candidate) => candidate.uri.toString() === uri.toString(),
          ),
        {
          createEditCommand: (title, edit) => this.controller.createEditCommand(title, edit, batch),
        },
      );
    } finally {
      batch.finish();
    }
  }

  async stop() {
    this.status = "stopped";
    this.stateListener?.dispose();
    await this.client?.stop();
  }
}
