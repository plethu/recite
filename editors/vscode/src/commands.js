import * as path from "node:path";
import { readCliConfiguration } from "./configuration.js";
import {
  savedSourceSnapshot,
  assertSavedSource,
  optionalSavePath,
  requiredBlock,
  requiredFixturePath,
  requiredOpenPath,
  requiredSavePath
} from "./command-inputs.js";
import { makeInvocationId } from "./command-process.js";
import { CommandProtocolError } from "./command-protocol.js";
import { WatchCommand } from "./watch-command.js";
import { replaceDiagnostics, clearDiagnostics } from "./command-diagnostics.js";
import { finiteCommand, executeFiniteCommand, replaceFiniteDiagnostics } from "./finite-commands.js";
import { disposeCommands, stopForAuthorityChange } from "./command-lifecycle.js";

const COMMANDS = Object.freeze([
  ["recite.validate", (registry, args) => registry.validate(args)],
  ["recite.compile", (registry, args) => registry.compile(args)],
  ["recite.extract", (registry, args) => registry.extract(args)],
  ["recite.watch.start", (registry, args) => registry.watchStart(args)],
  ["recite.watch.stop", (registry) => registry.watchStop()],
  ["recite.run", (registry, args) => registry.run(args)],
  ["recite.trace", (registry, args) => registry.trace(args)]
]);

const EMPTY_DIAGNOSTICS = Object.freeze({
  clear() {},
  set() {},
  delete() {},
  dispose() {}
});

export class CommandRegistry {
  constructor(api, userInterface, options = {}) {
    this.api = api;
    this.userInterface = userInterface;
    this.options = options;
    this.spawnProcess = options.spawnProcess;
    this.makeInvocationId = options.makeInvocationId ?? makeInvocationId;
    this.cliDiagnostics = api.languages?.createDiagnosticCollection?.("recite-cli") ?? EMPTY_DIAGNOSTICS;
    this.diagnosticUris = new Map();
    this.finiteGeneration = 0;
    this.finiteSessions = new Map();
    this.disposing = false;
    this.registered = false;
    this.watch = new WatchCommand(this);
  }

  register(subscriptions) {
    if (this.registered || !this.api.commands?.registerCommand) return;
    this.registered = true;
    for (const [id, callback] of COMMANDS) {
      subscriptions.push(this.api.commands.registerCommand(
        id,
        (args) => callback(this, args)
      ));
    }
  }

  async validate() {
    if (!this.trusted()) return undefined;
    let snapshot;
    try { snapshot = savedSourceSnapshot(this.userInterface); } catch (error) {
      this.failure(error);
      return undefined;
    }
    return this.finite("validate", [snapshot.path], { diagnostics: true }, undefined, snapshot);
  }

  async compile(args = {}) {
    if (!this.trusted()) return undefined;
    let snapshot, configuration, output;
    try {
      snapshot = savedSourceSnapshot(this.userInterface);
      configuration = this.configuration();
      output = await requiredSavePath(
        args?.output,
        this.userInterface,
        this.api.Uri?.file?.(path.join(configuration.projectRoot, "dialogue.recitec"))
      );
    } catch (error) {
      this.failure(error);
      return undefined;
    }
    if (!output) return undefined;
    if (!this.revalidateSource(snapshot, configuration)) return undefined;
    return this.finite("compile", ["--output", output, snapshot.path], { diagnostics: true }, configuration, snapshot);
  }

  async extract(args = {}) {
    if (!this.trusted()) return undefined;
    let snapshot, configuration, output;
    try {
      snapshot = savedSourceSnapshot(this.userInterface);
      configuration = this.configuration();
      output = await optionalSavePath(
        args?.output,
        this.userInterface,
        this.api.Uri?.file?.(path.join(configuration.projectRoot, "messages.pot"))
      );
    } catch (error) {
      this.failure(error);
      return undefined;
    }
    if (output === undefined) return undefined;
    if (!this.revalidateSource(snapshot, configuration)) return undefined;
    const commandArgs = output ? ["--output", output, snapshot.path] : [snapshot.path];
    return this.finite("extract", commandArgs, { diagnostics: true }, configuration, snapshot);
  }

  async run(args = {}) {
    return this.runtime("run", args);
  }

  async trace(args = {}) {
    return this.runtime("trace", args);
  }

  async runtime(command, args) {
    if (!this.trusted()) return undefined;
    let configuration, asset, block, fixture;
    try {
      configuration = this.configuration();
      asset = await requiredOpenPath(args?.asset, this.userInterface);
      block = await requiredBlock(args?.block, this.userInterface);
      fixture = await requiredFixturePath(args?.fixture, this.userInterface);
    } catch (error) {
      this.failure(error);
      return undefined;
    }
    if (!asset || !block || !fixture) return undefined;
    if (!this.trusted() || !this.configurationMatches(configuration)) return undefined;
    const invocationId = this.makeInvocationId();
    const commandArgs = [
      command, "--output-format", "structured", "--invocation-id", invocationId,
      asset, "--block", block, "--fixture", fixture
    ];
    return this.execute(configuration, command, invocationId, commandArgs, { diagnostics: false });
  }

  async watchStart(args = {}) {
    if (!this.trusted()) return undefined;
    let configuration;
    try {
      configuration = this.configuration();
    } catch (error) {
      this.failure(error);
      return undefined;
    }
    return this.watch.start(configuration, args);
  }

  async watchStop() {
    return this.watch.stop();
  }

  configuration() {
    return readCliConfiguration(this.api, this.userInterface);
  }

  configurationMatches(expected) {
    try {
      const current = this.configuration();
      return current.command === expected.command && current.cwd === expected.cwd &&
        current.projectRoot === expected.projectRoot &&
        current.projectRootOverridden === expected.projectRootOverridden;
    } catch (error) {
      this.failure(error);
      return false;
    }
  }

  trusted(notify = true) {
    if (this.disposing) return false;
    if (this.api.workspace.isTrusted === false) {
      if (notify) this.userInterface.commandNotTrusted();
      return false;
    }
    return true;
  }

  async finite(command, args, options, configurationOverride, sourceSnapshot) {
    return finiteCommand(this, command, args, options, configurationOverride, sourceSnapshot);
  }

  async execute(configuration, command, invocationId, args, options) {
    return executeFiniteCommand(this, configuration, command, invocationId, args, options);
  }

  replaceFiniteDiagnostics(session, records, projectRoot) {
    return replaceFiniteDiagnostics(this, session, records, projectRoot);
  }

  replaceWatchDiagnostics(session, records, projectRoot) {
    if (this.disposing || this.watch.active !== session) return;
    replaceDiagnostics(this.api, this.cliDiagnostics, records, projectRoot, this.diagnosticUris);
  }

  clearDiagnostics() {
    clearDiagnostics(this.cliDiagnostics, this.diagnosticUris);
  }

  failure(error) {
    if (error instanceof CommandProtocolError) {
      const detail = error.code;
      this.userInterface.commandProtocolFailure(detail);
      return;
    }
    const detail = error?.message ?? String(error);
    this.userInterface.commandFailure(detail);
  }

  revalidateSource(snapshot, configuration) {
    try {
      if (!this.configurationMatches(configuration)) return false;
      assertSavedSource(this.userInterface, snapshot);
      const relative = path.relative(configuration.projectRoot, snapshot.path);
      if (!relative || relative === ".." || relative.startsWith(`..${path.sep}`) || path.isAbsolute(relative)) {
        throw this.userInterface.commandDocumentOutsideRoot();
      }
      return true;
    } catch (error) {
      this.failure(error);
      return false;
    }
  }

  async dispose() {
    this.disposing = true;
    await disposeCommands(this);
  }

  /** Retire a watch before LSP/configuration authority changes. */
  async stopForAuthorityChange() {
    if (this.disposing) return undefined;
    return stopForAuthorityChange(this);
  }
}
