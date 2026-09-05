import {
  isValidLspRange,
  lspWorkspaceEditToVscode,
  workspaceEditStatus
} from "./lsp-features.js";

export const RENAME_BLOCK_COMMAND = "recite.renameBlock";

/**
 * The native VS Code rename provider cannot retain LSP document versions until
 * the host applies a WorkspaceEdit. Keep rename behind an explicit command so
 * the captured client, document generation, and every sibling precondition can
 * be checked at the only boundary where the edit is applied.
 */
export class RenameCommand {
  constructor(api, userInterface, getClient, options = {}) {
    this.api = api;
    this.userInterface = userInterface;
    this.getClient = getClient;
    this.getOpenDocument = options.getOpenDocument ?? ((uri) =>
      this.api.workspace.textDocuments?.find((document) =>
        document.uri.toString() === uri.toString()));
    this.active = undefined;
    this.disposed = false;
    this.registered = false;
  }

  register(subscriptions) {
    if (this.registered || !this.api.commands?.registerCommand) return;
    this.registered = true;
    subscriptions.push(this.api.commands.registerCommand(
      RENAME_BLOCK_COMMAND,
      () => this.execute()
    ));
  }

  async execute() {
    if (this.disposed) return false;
    if (this.active) {
      this.userInterface.renameBusy();
      return false;
    }
    const operation = this.run();
    this.active = operation;
    try {
      return await operation;
    } finally {
      if (this.active === operation) this.active = undefined;
    }
  }

  async run() {
    if (this.api.workspace.isTrusted === false) {
      this.userInterface.commandNotTrusted();
      return false;
    }

    const editor = this.userInterface.activeEditor() ?? this.api.window?.activeTextEditor;
    const document = editor?.document;
    const position = editor?.selection?.active;
    if (!document || document.languageId !== "recite" || !position ||
      !Number.isInteger(position.line) || !Number.isInteger(position.character)) {
      this.userInterface.renameDocumentRequired();
      return false;
    }

    const uri = document.uri?.toString?.();
    if (!uri || !Number.isInteger(document.version)) {
      this.userInterface.renameUnavailable();
      return false;
    }

    const client = this.getClient();
    if (!client || client.status !== "running" || typeof client.request !== "function") {
      const error = this.userInterface.serverNotRunning();
      const detail = error?.message ?? String(error);
      this.userInterface.commandFailure(detail);
      return false;
    }

    const snapshot = {
      document,
      uri,
      version: document.version,
      position: { line: position.line, character: position.character }
    };
    const current = () => this.isCurrent(snapshot, client);
    const params = {
      textDocument: { uri: snapshot.uri },
      position: snapshot.position
    };

    let prepared;
    try {
      prepared = await client.request("textDocument/prepareRename", params);
    } catch (error) {
      if (!current()) return this.stale();
      const detail = error?.message ?? String(error);
      this.userInterface.renameRequestFailed(detail);
      return false;
    }
    if (!current()) return this.stale();

    const preparation = classifyPrepareRename(prepared);
    if (preparation.kind === "none") {
      this.userInterface.renameUnavailable();
      return false;
    }
    if (preparation.kind === "invalid") {
      this.userInterface.renameInvalid();
      return false;
    }

    let newName;
    try {
      const placeholder = preparation.placeholder;
      newName = await this.userInterface.chooseRenameName(placeholder);
    } catch (error) {
      if (!current()) return this.stale();
      const detail = error?.message ?? String(error);
      this.userInterface.renameRequestFailed(detail);
      return false;
    }
    if (!current()) return this.stale();
    // An undefined input is the VS Code input-box cancellation result. It is a
    // deliberate no-op and must not be reported as a failure.
    if (newName === undefined) return undefined;
    if (typeof newName !== "string") {
      this.userInterface.renameInvalid();
      return false;
    }

    let result;
    try {
      result = await client.request("textDocument/rename", {
        ...params,
        newName
      });
    } catch (error) {
      if (!current()) return this.stale();
      const detail = error?.message ?? String(error);
      this.userInterface.renameRequestFailed(detail);
      return false;
    }
    if (!current()) return this.stale();

    // Rename is intentionally narrower than a general WorkspaceEdit: it must
    // contain at least one versioned document change. The conversion helper
    // then requires every affected document to be open at that exact version.
    if (!Array.isArray(result?.documentChanges) || result.documentChanges.length === 0 ||
      Object.hasOwn(result, "changes")) {
      this.userInterface.renameInvalid();
      return false;
    }
    let edit;
    try {
      edit = lspWorkspaceEditToVscode(this.api, result, this.getOpenDocument);
    } catch {
      edit = undefined;
    }
    if (!edit) {
      this.userInterface.renameUnavailable();
      return false;
    }

    // Keep this check immediately before the version status check. There must
    // be no await or host callback between status validation and applyEdit.
    if (!current()) return this.stale();
    const status = workspaceEditStatus(edit);
    if (status !== "current") {
      this.userInterface.renameStale();
      return false;
    }
    let applied;
    try {
      applied = await this.api.workspace.applyEdit(edit);
    } catch {
      if (this.disposed) return false;
      this.userInterface.renameApplyFailed();
      return false;
    }
    if (this.disposed) return false;
    if (applied === false) {
      this.userInterface.renameApplyFailed();
      return false;
    }
    return applied;
  }

  isCurrent(snapshot, client) {
    if (this.disposed || this.getClient() !== client || client.status !== "running") return false;
    const editor = this.userInterface.activeEditor() ?? this.api.window?.activeTextEditor;
    const document = editor?.document;
    const position = editor?.selection?.active;
    return document === snapshot.document &&
      document?.uri?.toString?.() === snapshot.uri &&
      document.version === snapshot.version &&
      position?.line === snapshot.position.line &&
      position?.character === snapshot.position.character &&
      this.userInterface.documentIsOpen(document);
  }

  stale() {
    if (!this.disposed) this.userInterface.renameStale();
    return false;
  }

  dispose() {
    this.disposed = true;
    // Requests do not advertise LSP cancellation yet. Let an in-flight request
    // settle, but all of its post-await guards become false immediately.
  }
}

function classifyPrepareRename(result) {
  if (result === null || result === undefined) return { kind: "none" };
  if (typeof result !== "object" || Array.isArray(result)) return { kind: "invalid" };
  if (hasExactKeys(result, ["start", "end"]) && validRange(result)) {
    return { kind: "valid" };
  }
  if (hasExactKeys(result, ["range", "placeholder"]) && validRange(result.range) &&
    typeof result.placeholder === "string") {
    return { kind: "valid", placeholder: result.placeholder };
  }
  if (hasExactKeys(result, ["defaultBehavior"]) && result.defaultBehavior === true) {
    return { kind: "valid" };
  }
  return { kind: "invalid" };
}

function validRange(range) {
  return Boolean(range && typeof range === "object" && !Array.isArray(range) &&
    hasExactKeys(range, ["start", "end"]) && isValidLspRange(range));
}

function hasExactKeys(value, expected) {
  const keys = Object.keys(value).sort();
  return keys.length === expected.length && expected.slice().sort().every((key, index) => key === keys[index]);
}
