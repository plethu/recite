import { is, commandPathSchema } from "./schemas/index.js";
import type { UserInterface, SourceSnapshot } from "./types.ts";
import * as path from "node:path";

export function savedSourceSnapshot(userInterface: UserInterface) {
  const document = userInterface.activeDocument();
  if (!document || document.languageId !== "recite") {
    throw userInterface.commandDocumentRequired();
  }
  if (document.isUntitled || document.uri?.scheme !== "file") {
    throw userInterface.commandUntitledDocument();
  }
  if (document.isDirty) throw userInterface.commandDocumentUnsaved();
  return {
    path: document.uri.fsPath,
    uri: document.uri.toString?.() ?? document.uri.fsPath,
    version: document.version,
    document,
  };
}

export function assertSavedSource(userInterface: UserInterface, snapshot: SourceSnapshot) {
  const document = snapshot.document;
  if (
    !document ||
    document.isUntitled ||
    document.isDirty ||
    document.version !== snapshot.version ||
    document.uri?.scheme !== "file" ||
    document.uri.toString?.() !== snapshot.uri ||
    document.uri.fsPath !== snapshot.path ||
    !userInterface.documentIsOpen(document)
  ) {
    throw userInterface.commandDocumentChanged();
  }
  return { ...snapshot, document };
}

export async function requiredSavePath(
  provided: unknown,
  userInterface: UserInterface,
  defaultUri: import("vscode").Uri | undefined,
) {
  if (provided !== undefined) return commandPath(provided, userInterface);
  const selected = await userInterface.chooseCompileOutputPath(defaultUri);
  if (!selected) return undefined;
  return commandPath(selected, userInterface);
}

export async function optionalSavePath(
  provided: unknown,
  userInterface: UserInterface,
  defaultUri: import("vscode").Uri | undefined,
) {
  if (provided !== undefined) {
    return provided === null ? null : commandPath(provided, userInterface);
  }
  const selected = await userInterface.chooseExtractOutputPath(defaultUri);
  return selected ? commandPath(selected, userInterface) : undefined;
}

export async function requiredOpenPath(provided: unknown, userInterface: UserInterface) {
  if (provided !== undefined) return commandPath(provided, userInterface);
  const selected = await userInterface.chooseAssetPath();
  return selected?.[0] ? commandPath(selected[0], userInterface) : undefined;
}

export async function requiredBlock(provided: unknown, userInterface: UserInterface) {
  if (provided !== undefined) return commandValue(provided, userInterface);
  const block = await userInterface.chooseBlock();
  return block === undefined ? undefined : commandValue(block, userInterface);
}

export async function requiredFixturePath(provided: unknown, userInterface: UserInterface) {
  if (provided !== undefined) return commandPath(provided, userInterface);
  const selected = await userInterface.chooseFixturePath();
  return selected?.[0] ? commandPath(selected[0], userInterface) : undefined;
}

function commandPath(value: unknown, userInterface: UserInterface) {
  const candidate =
    typeof value === "string" ? value : is(commandPathSchema, value) ? value.fsPath : undefined;
  if (typeof candidate !== "string" || candidate.trim() === "" || !path.isAbsolute(candidate)) {
    throw userInterface.commandInputInvalid();
  }
  return path.normalize(candidate);
}

function commandValue(value: unknown, userInterface: UserInterface) {
  if (typeof value !== "string" || value.trim() === "") throw userInterface.commandInputInvalid();
  return value;
}
