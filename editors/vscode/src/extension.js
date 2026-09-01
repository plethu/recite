import * as vscode from "vscode";
import { ExtensionController } from "./controller.js";

let activeController;

export async function activate(context) {
  const output = vscode.window.createOutputChannel("Recite");
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
