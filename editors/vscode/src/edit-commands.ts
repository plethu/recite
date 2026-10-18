import type {
  HostApi,
  UserInterface,
  EditorOptions,
  Timer,
  VersionedEdit,
  EditBatch,
  EditFailure,
} from "./types.ts";
import { workspaceEditStatus } from "./lsp-features.ts";

const APPLY_CODE_ACTION_COMMAND = "recite.applyCodeAction";
const EDIT_COMMAND_TTL_MS = 30_000;
const MAX_EDIT_COMMANDS = 128;
const MAX_RETIRED_COMMANDS = 128;

export class EditCommandRegistry {
  declare api: HostApi;
  declare userInterface: UserInterface;
  declare ttlMs: number;
  declare maxCommands: number;
  declare retiredTtlMs: number;
  declare maxRetiredCommands: number;
  declare commands: Map<string, { edit: VersionedEdit; expiresAt: number }>;
  declare retired: Map<string, { reason: EditFailure; expiresAt: number }>;
  declare nextId: number;
  declare expiryTimer: Timer | undefined;
  declare registered: boolean;

  constructor(api: HostApi, userInterface: UserInterface, options: EditorOptions = {}) {
    this.api = api;
    this.userInterface = userInterface;
    this.ttlMs = Math.max(1, options.editCommandTtlMs ?? EDIT_COMMAND_TTL_MS);
    this.maxCommands = Math.max(1, Math.floor(options.maxEditCommands ?? MAX_EDIT_COMMANDS));
    this.retiredTtlMs = Math.max(1, options.retiredCommandTtlMs ?? this.ttlMs);
    this.maxRetiredCommands = Math.max(
      1,
      Math.min(MAX_RETIRED_COMMANDS, Math.floor(options.maxRetiredCommands ?? this.maxCommands)),
    );
    this.commands = new Map();
    this.retired = new Map();
    this.nextId = 1;
    this.expiryTimer = undefined;
    this.registered = false;
  }

  register(subscriptions: import("vscode").Disposable[]) {
    if (this.registered || !this.api.commands?.registerCommand) return;
    this.registered = true;
    subscriptions.push(
      this.api.commands.registerCommand(APPLY_CODE_ACTION_COMMAND, (id: unknown) => this.apply(id)),
    );
  }

  beginBatch() {
    const batch: EditBatch = {
      ids: new Set<string>(),
      active: true,
      finish: () => {
        batch.active = false;
      },
    };
    return batch;
  }

  create(title: string, edit: VersionedEdit, batch: EditBatch | undefined) {
    if (!this.registered || (batch && !batch.active)) return undefined;
    this.prune();
    while (this.commands.size >= this.maxCommands) {
      const oldest = this.commands.keys().next().value!;
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

  apply(id: unknown) {
    this.prune();
    const key = String(id);
    const entry = this.commands.get(key);
    if (!entry) {
      this.reportFailure(this.retired.get(key)?.reason ?? "unknown");
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

  discardForDocument(
    document: import("vscode").TextDocument,
    reason: EditFailure = "document-stale",
  ) {
    for (const [id, entry] of this.commands) {
      if (
        entry.edit.reciteVersionPreconditions?.some(
          (precondition) => precondition.document === document,
        )
      )
        this.retire(id, reason);
    }
    this.schedulePrune();
  }

  prune() {
    const now = Date.now();
    for (const [id, entry] of this.commands) {
      if (entry.expiresAt <= now) this.retire(id, "expired");
    }
    for (const [id, entry] of this.retired) {
      if (entry.expiresAt <= now) this.retired.delete(id);
    }
    this.schedulePrune();
  }

  schedulePrune() {
    if (this.expiryTimer) clearTimeout(this.expiryTimer);
    this.expiryTimer = undefined;
    const nextExpiry = [...this.commands.values(), ...this.retired.values()].reduce(
      (next, entry) => Math.min(next, entry.expiresAt),
      Infinity,
    );
    if (!Number.isFinite(nextExpiry)) return;
    this.expiryTimer = setTimeout(
      () => {
        this.expiryTimer = undefined;
        this.prune();
      },
      Math.max(0, nextExpiry - Date.now()),
    );
    this.expiryTimer.unref?.();
  }

  retire(id: string, reason: EditFailure) {
    this.commands.delete(id);
    this.retired.set(id, { reason, expiresAt: Date.now() + this.retiredTtlMs });
    while (this.retired.size > this.maxRetiredCommands) {
      this.retired.delete(this.retired.keys().next().value!);
    }
  }

  reportFailure(reason: EditFailure) {
    switch (reason) {
      case "document-stale":
        this.userInterface.actionStale();
        break;
      case "document-closed":
        this.userInterface.actionClosed();
        break;
      case "document-reopened":
        this.userInterface.actionReopened();
        break;
      case "expired":
        this.userInterface.actionExpired();
        break;
      case "evicted":
        this.userInterface.actionEvicted();
        break;
      case "apply-failed":
        this.userInterface.actionApplyFailed();
        break;
      default:
        this.userInterface.actionUnknown();
    }
  }

  dispose() {
    if (this.expiryTimer) clearTimeout(this.expiryTimer);
    this.expiryTimer = undefined;
    this.commands.clear();
    this.retired.clear();
  }
}
