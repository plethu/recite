import * as vscode from "vscode";
import { ExtensionController } from "./controller.js";
import { createUserInterface } from "./user-interface.js";

let activeController;

export async function activate(context) {
  const userInterface = createUserInterface(vscode);
  const diagnostics = vscode.languages.createDiagnosticCollection("recite");
  const controller = new ExtensionController(vscode, userInterface, diagnostics);
  activeController = controller;
  context.subscriptions.push(userInterface, diagnostics, controller);
  await controller.start().catch((error) => controller.handleStartFailure(error));
}

export async function deactivate() {
  const controller = activeController;
  activeController = undefined;
  await controller?.dispose();
}
