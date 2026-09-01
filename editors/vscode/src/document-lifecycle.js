const RECITE_LANGUAGE_ID = "recite";

export function registerDocumentLifecycle(controller) {
  const { api, subscriptions } = controller;
  for (const document of api.workspace.textDocuments ?? []) controller.open(document);

  subscriptions.push(
    api.workspace.onDidOpenTextDocument((document) => controller.open(document)),
    api.workspace.onDidChangeTextDocument((event) => {
      const { document } = event;
      if (!isReciteDocument(document)) return;
      controller.discardEditCommandsForDocument(document);
      controller.documents.set(document.uri.toString(), document);
      if (controller.client?.status !== "running") return;
      controller.client.notify("textDocument/didChange", {
        textDocument: { uri: document.uri.toString(), version: document.version },
        contentChanges: [{ text: document.getText() }]
      });
    }),
    api.workspace.onDidSaveTextDocument((document) => {
      if (!isReciteDocument(document) || controller.client?.status !== "running") return;
      controller.client.notify("textDocument/didSave", {
        textDocument: { uri: document.uri.toString() }
      });
    }),
    api.workspace.onDidCloseTextDocument((document) => {
      if (!isReciteDocument(document)) return;
      controller.discardEditCommandsForDocument(document);
      const uri = document.uri.toString();
      controller.documents.delete(uri);
      if (controller.client?.status === "running") {
        controller.client.notify("textDocument/didClose", { textDocument: { uri } });
      }
      controller.diagnostics.delete(document.uri);
    }),
    api.workspace.onDidChangeConfiguration((event) => {
      if (!event.affectsConfiguration("recite.lsp")) return;
      void controller.restart().catch((error) => controller.handleStartFailure(error));
    })
  );
}

export function isReciteDocument(document) {
  return document.languageId === RECITE_LANGUAGE_ID;
}
