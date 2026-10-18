import { ExtensionController } from "./controller.ts";
import { createUserInterface } from "./user-interface.ts";
import type { HostApi } from "./types.ts";
import type { ExtensionContext } from "vscode";

let activeController: ExtensionController | undefined;

export async function activateWithVscode(vscode: HostApi, context: ExtensionContext) {
  const userInterface = createUserInterface(vscode);
  const controller = new ExtensionController(vscode, userInterface);
  activeController = controller;
  context.subscriptions.push(userInterface, controller);
  const outcome = await controller
    .start()
    .catch((error) => controller.handleUnexpectedStartFailure(error));
  if (outcome) controller.handleStartOutcome(outcome);
}

export async function deactivateWithVscode() {
  const controller = activeController;
  activeController = undefined;
  await controller?.dispose();
}
