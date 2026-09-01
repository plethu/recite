import * as vscode from "vscode";
import { ExtensionController } from "./controller.js";
import { clientMessage } from "./messages.js";

let activeController;

export async function activate(context) {
  const output = vscode.window.createOutputChannel(clientMessage(vscode, "lsp-client-display-name"));
  const diagnostics = vscode.languages.createDiagnosticCollection("recite");
  const controller = new ExtensionController(vscode, output, diagnostics);
  activeController = controller;
  context.subscriptions.push(output, diagnostics, controller);
  await controller.start().catch((error) => controller.handleStartFailure(error));
}

export async function deactivate() {
  const controller = activeController;
  activeController = undefined;
  await controller?.dispose();
}
