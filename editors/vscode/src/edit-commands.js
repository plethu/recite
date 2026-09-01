import { workspaceEditStatus } from "./lsp-features.js";
import { clientMessage } from "./messages.js";

const APPLY_CODE_ACTION_COMMAND = "recite.applyCodeAction";
const EDIT_COMMAND_TTL_MS = 30_000;
const MAX_EDIT_COMMANDS = 128;

export class EditCommandRegistry {
  constructor(api, output, options = {}) {
    this.api = api;
    this.output = output;
    this.ttlMs = Math.max(1, options.editCommandTtlMs ?? EDIT_COMMAND_TTL_MS);
    this.maxCommands = Math.max(1, Math.floor(options.maxEditCommands ?? MAX_EDIT_COMMANDS));
    this.commands = new Map();
    this.retired = new Map();
    this.nextId = 1;
    this.expiryTimer = undefined;
    this.registered = false;
  }

  register(subscriptions) {
    if (this.registered || !this.api.commands?.registerCommand) return;
    this.registered = true;
    subscriptions.push(this.api.commands.registerCommand(
      APPLY_CODE_ACTION_COMMAND,
      (id) => this.apply(id)
    ));
  }

  beginBatch() {
    const batch = { ids: new Set(), active: true };
    batch.finish = () => { batch.active = false; };
    return batch;
  }

  create(title, edit, batch) {
    if (!this.registered || (batch && !batch.active)) return undefined;
    this.prune();
    while (this.commands.size >= this.maxCommands) {
      const oldest = this.commands.keys().next().value;
      if (batch?.ids.has(oldest)) return undefined;
      this.retire(oldest, "evicted");
    }
    const id = String(this.nextId++);
    this.commands.set(id, { edit, expiresAt: Date.now() + this.ttlMs });
    batch?.ids.add(id);
    this.schedulePrune();
    if (this.api.Command) return new this.api.Command(title, APPLY_CODE_ACTION_COMMAND, id);
    return { title, command: APPLY_CODE_ACTION_COMMAND, arguments: [id] };
  }

  apply(id) {
    this.prune();
    const key = String(id);
    const entry = this.commands.get(key);
    if (!entry) {
      this.reportFailure(this.retired.get(key) ?? "unknown");
      return false;
    }
    this.commands.delete(key);
    this.schedulePrune();
    const status = workspaceEditStatus(entry.edit);
    if (status !== "current") {
      this.reportFailure(status);
      return false;
    }
    const result = this.api.workspace.applyEdit(entry.edit);
    return Promise.resolve(result).then((applied) => {
      if (applied === false) {
        this.reportFailure("apply-failed");
        return false;
      }
      return applied;
    });
  }

  discardForDocument(document, reason = "document-stale") {
    for (const [id, entry] of this.commands) {
      if (entry.edit.reciteVersionPreconditions?.some((precondition) =>
        precondition.document === document
      )) this.retire(id, reason);
    }
    this.schedulePrune();
  }

  prune() {
    const now = Date.now();
    for (const [id, entry] of this.commands) {
      if (entry.expiresAt <= now) this.retire(id, "expired");
    }
    this.schedulePrune();
  }

  schedulePrune() {
    if (this.expiryTimer) clearTimeout(this.expiryTimer);
    this.expiryTimer = undefined;
    const nextExpiry = [...this.commands.values()]
      .reduce((next, entry) => Math.min(next, entry.expiresAt), Infinity);
    if (!Number.isFinite(nextExpiry)) return;
    this.expiryTimer = setTimeout(() => {
      this.expiryTimer = undefined;
      this.prune();
    }, Math.max(0, nextExpiry - Date.now()));
    this.expiryTimer.unref?.();
  }

  retire(id, reason) {
    this.commands.delete(id);
    this.retired.set(id, reason);
  }

  reportFailure(reason) {
    switch (reason) {
      case "document-stale":
        this.output.appendLine(clientMessage(this.api, "lsp-client-action-stale"));
        break;
      case "document-closed":
        this.output.appendLine(clientMessage(this.api, "lsp-client-action-closed"));
        break;
      case "document-reopened":
        this.output.appendLine(clientMessage(this.api, "lsp-client-action-reopened"));
        break;
      case "expired":
        this.output.appendLine(clientMessage(this.api, "lsp-client-action-expired"));
        break;
      case "evicted":
        this.output.appendLine(clientMessage(this.api, "lsp-client-action-evicted"));
        break;
      case "apply-failed":
        this.output.appendLine(clientMessage(this.api, "lsp-client-action-apply-failed"));
        break;
      default:
        this.output.appendLine(clientMessage(this.api, "lsp-client-action-unknown"));
    }
  }

  dispose() {
    if (this.expiryTimer) clearTimeout(this.expiryTimer);
    this.expiryTimer = undefined;
    this.commands.clear();
    this.retired.clear();
  }
}
