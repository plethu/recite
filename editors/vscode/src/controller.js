import { ReciteStandardClient } from "./standard-client.js";
import { readConfiguration } from "./configuration.js";
import { EditCommandRegistry } from "./edit-commands.js";
import { RestartPolicy } from "./restart-policy.js";
import { StartupOutcomeKind, startupOutcome } from "./startup-outcome.js";
import { CommandRegistry } from "./commands.js";
import { RenameCommand } from "./rename-command.js";

const STABLE_RUN_MS = 10_000;

export class ExtensionController {
  constructor(api, userInterface, _diagnostics, options = {}) {
    this.api = api;
    this.userInterface = userInterface;
    this.createClient = options.createClient ?? ((configuration) =>
      new ReciteStandardClient(api, configuration, this));
    this.stableRunMs = options.stableRunMs ?? STABLE_RUN_MS;
    this.restartPolicy = new RestartPolicy(options.restartDelaysMs);
    this.client = undefined;
    this.restartPromise = undefined;
    this.restartRevision = 0;
    this.restartTimer = undefined;
    this.stableRunTimer = undefined;
    this.stopping = false;
    this.disposed = false;
    this.subscriptions = [];
    this.registered = false;
    this.editCommands = new EditCommandRegistry(api, userInterface, options);
    this.commands = new CommandRegistry(api, userInterface, options);
    this.renameCommand = new RenameCommand(api, userInterface, () => this.client, options);
  }

  async start(phase = "initial") {
    this.ensureSubscriptions();
    if (this.disposed || this.api.workspace.isTrusted === false) {
      return startupOutcome(StartupOutcomeKind.Refused);
    }
    if (this.client?.status === "running" || this.client?.status === "starting") {
      return startupOutcome(StartupOutcomeKind.Started);
    }
    if (this.client?.status === "stopped") {
      const stopped = this.client;
      this.retireClient(stopped);
      try { await stopped.stop(); } catch { /* already stopped */ }
    }
    let configuration;
    try {
      configuration = readConfiguration(this.api, this.userInterface);
    } catch (error) {
      return startupOutcome(StartupOutcomeKind.Refused, error);
    }
    let client;
    try {
      client = this.createClient(configuration);
    } catch (error) {
      return startupOutcome(StartupOutcomeKind.RetryableFailure, error);
    }
    this.client = client;
    client.on("failure", (failure) => this.handleClientFailure(client, failure, phase));
    client.on("exit", () => this.handleExit(client, phase));
    try {
      await client.start();
      if (this.disposed || this.client !== client || client.retired || client.status !== "running") {
        try { await client.stop(); } catch { /* already stopped */ }
        return startupOutcome(StartupOutcomeKind.Refused);
      }
      this.scheduleStableReset(client);
      return startupOutcome(StartupOutcomeKind.Started);
    } catch (error) {
      this.retireClient(client);
      return startupOutcome(StartupOutcomeKind.RetryableFailure, error, client.failureReported);
    }
  }

  ensureSubscriptions() {
    if (this.registered) return;
    this.registered = true;
    const workspace = this.api.workspace;
    if (workspace.onDidGrantWorkspaceTrust) {
      this.subscriptions.push(workspace.onDidGrantWorkspaceTrust(() => {
        void this.start().then((outcome) => this.handleStartOutcome(outcome))
          .catch((error) => this.handleUnexpectedStartFailure(error));
      }));
    }
    this.subscriptions.push(
      workspace.onDidChangeTextDocument((event) =>
        this.editCommands.discardForDocument(event.document, "document-stale")),
      workspace.onDidCloseTextDocument((document) =>
        this.editCommands.discardForDocument(document, "document-closed")),
      workspace.onDidChangeConfiguration((event) => {
        if (event.affectsConfiguration("recite.lsp") || event.affectsConfiguration("recite.cli")) {
          void this.restart().catch((error) => this.handleUnexpectedStartFailure(error));
        }
      })
    );
    if (workspace.onDidChangeWorkspaceFolders) {
      this.subscriptions.push(workspace.onDidChangeWorkspaceFolders(() =>
        void this.restart().catch((error) => this.handleUnexpectedStartFailure(error))));
    }
    this.editCommands.register(this.subscriptions);
    this.commands.register(this.subscriptions);
    this.renameCommand.register(this.subscriptions);
  }

  createEditCommand(title, edit, batch) {
    return this.editCommands.create(title, edit, batch);
  }
  createEditCommandBatch() {
    return this.editCommands.beginBatch();
  }

  handleStartOutcome(outcome, phase = "initial") {
    if (outcome.kind === StartupOutcomeKind.Refused) {
      if (outcome.error) {
        const detail = outcome.error.message ?? String(outcome.error);
        this.userInterface.serverLifecycleFailure(detail);
      }
      return outcome;
    }
    if (outcome.kind === StartupOutcomeKind.RetryableFailure) {
      if (phase !== "retrying" && !outcome.reported && outcome.error) {
        const detail = outcome.error.message ?? String(outcome.error);
        this.userInterface.serverLifecycleFailure(detail);
      }
      this.scheduleRestart();
    }
    return outcome;
  }
  handleUnexpectedStartFailure(error) {
    const detail = error?.message ?? String(error);
    this.userInterface.serverLifecycleFailure(detail);
    this.scheduleRestart();
  }
  handleClientFailure(client, error, phase) {
    if (this.client !== client || client.retired) return;
    client.failureReported = true;
    if (phase !== "retrying") {
      const detail = error?.message ?? String(error);
      this.userInterface.serverLifecycleFailure(detail);
    }
    if (client.status === "stopped") {
      this.retireClient(client, false);
      this.scheduleRestart();
    }
  }
  handleExit(client, phase) {
    if (this.client !== client || client.retired || this.stopping || this.disposed) return;
    this.clearStableReset();
    this.retireClient(client);
    if (phase !== "retrying" && !client.failureReported) this.userInterface.serverExited();
    this.scheduleRestart();
  }
  retireClient(client, clearReference = true) {
    if (!client) return;
    client.retired = true;
    if (clearReference && this.client === client) this.client = undefined;
  }
  scheduleRestart() {
    if (this.restartTimer || this.disposed || this.stopping || this.api.workspace.isTrusted === false) return;
    const delay = this.restartPolicy.nextDelay();
    if (delay === undefined) {
      if (this.restartPolicy.reportExhausted()) this.userInterface.restartExhausted();
      return;
    }
    this.userInterface.restartScheduled(delay);
    this.restartTimer = setTimeout(() => {
      this.restartTimer = undefined;
      void this.start("retrying")
        .then((outcome) => this.handleStartOutcome(outcome, "retrying"))
        .catch(() => this.scheduleRestart());
    }, delay);
    this.restartTimer.unref?.();
  }
  scheduleStableReset(client) {
    this.clearStableReset();
    this.stableRunTimer = setTimeout(() => {
      if (this.client === client && client.status === "running") this.restartPolicy.reset();
      this.stableRunTimer = undefined;
    }, this.stableRunMs);
    this.stableRunTimer.unref?.();
  }
  clearStableReset() {
    if (this.stableRunTimer) clearTimeout(this.stableRunTimer);
    this.stableRunTimer = undefined;
  }
  async restart() {
    if (!this.restartPromise) this.restartPolicy.reset();
    this.restartRevision += 1;
    if (this.restartTimer) clearTimeout(this.restartTimer);
    this.restartTimer = undefined;
    if (this.restartPromise) return this.restartPromise;
    this.restartPromise = (async () => {
      let revision;
      do {
        this.stopping = true;
        this.clearStableReset();
        try {
          await this.commands.stopForAuthorityChange();
          const retired = this.client;
          this.retireClient(retired);
          await retired?.stop();
        } finally {
          this.stopping = false;
        }
        revision = this.restartRevision;
        const outcome = await this.start();
        this.handleStartOutcome(outcome);
        if (outcome.kind !== StartupOutcomeKind.Started) break;
      } while (this.restartRevision !== revision && !this.disposed);
    })().finally(() => { this.restartPromise = undefined; });
    return this.restartPromise;
  }
  async dispose() {
    this.disposed = true;
    this.stopping = true;
    if (this.restartTimer) clearTimeout(this.restartTimer);
    this.restartTimer = undefined;
    this.clearStableReset();
    this.editCommands.dispose();
    await this.renameCommand.dispose();
    await this.commands.dispose();
    for (const subscription of this.subscriptions.splice(0)) subscription.dispose();
    const retired = this.client;
    this.retireClient(retired);
    try { await retired?.stop(); } finally { this.client = undefined; }
  }
}
