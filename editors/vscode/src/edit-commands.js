import { workspaceEditIsCurrent } from "./lsp-features.js";
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

  create(title, edit) {
    if (!this.registered) return undefined;
    this.prune();
    while (this.commands.size >= this.maxCommands) {
      this.commands.delete(this.commands.keys().next().value);
    }
    const id = String(this.nextId++);
    this.commands.set(id, { edit, expiresAt: Date.now() + this.ttlMs });
    this.schedulePrune();
    if (this.api.Command) return new this.api.Command(title, APPLY_CODE_ACTION_COMMAND, id);
    return { title, command: APPLY_CODE_ACTION_COMMAND, arguments: [id] };
  }

  apply(id) {
    this.prune();
    const entry = this.commands.get(id);
    this.commands.delete(id);
    if (!entry || !workspaceEditIsCurrent(entry.edit)) {
      this.output.appendLine(clientMessage(this.api, "lsp-client-action-stale"));
      return false;
    }
    return this.api.workspace.applyEdit(entry.edit);
  }

  discardForDocument(document) {
    for (const [id, entry] of this.commands) {
      if (entry.edit.reciteVersionPreconditions?.some((precondition) =>
        precondition.document === document
      )) this.commands.delete(id);
    }
    this.schedulePrune();
  }

  prune() {
    const now = Date.now();
    for (const [id, entry] of this.commands) {
      if (entry.expiresAt <= now) this.commands.delete(id);
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

  dispose() {
    if (this.expiryTimer) clearTimeout(this.expiryTimer);
    this.expiryTimer = undefined;
    this.commands.clear();
  }
}
