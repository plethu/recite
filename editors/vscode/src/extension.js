import { ExtensionController } from "./controller.js";
import { createUserInterface } from "./user-interface.js";

let activeController;

export async function activateWithVscode(vscode, context) {
  const userInterface = createUserInterface(vscode);
  const diagnostics = vscode.languages.createDiagnosticCollection("recite");
  const controller = new ExtensionController(vscode, userInterface, diagnostics);
  activeController = controller;
  context.subscriptions.push(userInterface, diagnostics, controller);
  const outcome = await controller.start().catch((error) => controller.handleUnexpectedStartFailure(error));
  if (outcome) controller.handleStartOutcome(outcome);
}

export async function deactivateWithVscode() {
  const controller = activeController;
  activeController = undefined;
  await controller?.dispose();
}
