import {
  lspCodeActionsToVscode,
  lspCompletionItems,
  lspHoverToVscode,
  lspLocationToVscode,
  vscodeDiagnosticToLsp,
} from "./lsp-features.js";

const SELECTOR = [
  { scheme: "file", language: "recite" },
  { scheme: "untitled", language: "recite" }
];

export function registerFeatureProviders(controller) {
  const { api, subscriptions } = controller;
  const send = (method, params, token) => {
    const client = controller.client;
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
  const getDocument = (uri) => api.workspace.textDocuments.find(
    (document) => document.uri.toString() === uri.toString()
  );
  const provider = (callback, method, transform) => ({
    [callback]: (document, position, token, extra) => request(method, document, position, token, extra)
      .then((result) => transform(result))
  });

  subscriptions.push(
    api.languages.registerCompletionItemProvider(
      SELECTOR,
      provider("provideCompletionItems", "textDocument/completion", (result) =>
        lspCompletionItems(api, result)),
      "(", "="
    ),
    api.languages.registerHoverProvider(
      SELECTOR,
      provider("provideHover", "textDocument/hover", (result) => lspHoverToVscode(api, result))
    ),
    api.languages.registerDefinitionProvider(
      SELECTOR,
      provider("provideDefinition", "textDocument/definition", (result) => locations(api, result))
    ),
    api.languages.registerReferenceProvider(SELECTOR, {
      provideReferences: (document, position, context, token) => request(
        "textDocument/references", document, position, token,
        { context: { includeDeclaration: context.includeDeclaration } }
      ).then((result) => locations(api, result))
    }),
    api.languages.registerCodeActionsProvider(SELECTOR, {
      provideCodeActions: (document, range, context, token) => send("textDocument/codeAction", {
        textDocument: { uri: document.uri.toString() },
        range: {
          start: { line: range.start.line, character: range.start.character },
          end: { line: range.end.line, character: range.end.character }
        },
        context: { diagnostics: context.diagnostics.map((diagnostic) =>
          vscodeDiagnosticToLsp(api, diagnostic)
        ) }
      }, token).then((result) => lspCodeActionsToVscode(api, result, getDocument, {
        createEditCommand: (title, edit) => controller.createEditCommand(title, edit)
      }))
    })
  );
}

function locations(api, result) {
  const values = Array.isArray(result) ? result : result ? [result] : [];
  return values.map((location) => lspLocationToVscode(api, location)).filter(Boolean);
}
