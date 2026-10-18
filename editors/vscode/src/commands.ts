type CommandArguments = { output?: unknown; asset?: unknown; block?: unknown; fixture?: unknown };
import { errorMessage } from "./errors.ts";
import type {
  HostApi,
  UserInterface,
  EditorOptions,
  FiniteSession,
  CliConfiguration,
  SourceSnapshot,
  WatchSession,
} from "./types.ts";
import * as path from "node:path";
import { clearDiagnostics, replaceDiagnostics } from "./command-diagnostics.ts";
import {
  assertSavedSource,
  optionalSavePath,
  requiredBlock,
  requiredFixturePath,
  requiredOpenPath,
  requiredSavePath,
  savedSourceSnapshot,
} from "./command-inputs.ts";
import { disposeCommands, stopForAuthorityChange } from "./command-lifecycle.ts";
import { makeInvocationId } from "./command-process.ts";
import { CommandProtocolError } from "./command-protocol.ts";
import { readCliConfiguration } from "./configuration.ts";
import {
  executeFiniteCommand,
  finiteCommand,
  replaceFiniteDiagnostics,
} from "./finite-commands.ts";
import { WatchCommand } from "./watch-command.ts";

const COMMANDS: ReadonlyArray<
  readonly [string, (registry: CommandRegistry, args: CommandArguments) => Promise<unknown>]
> = Object.freeze([
  ["recite.validate", (registry) => registry.validate()],
  ["recite.compile", (registry, args: CommandArguments) => registry.compile(args)],
  ["recite.extract", (registry, args: CommandArguments) => registry.extract(args)],
  ["recite.watch.start", (registry, args: CommandArguments) => registry.watchStart(args)],
  ["recite.watch.stop", (registry) => registry.watchStop()],
  ["recite.run", (registry, args: CommandArguments) => registry.run(args)],
  ["recite.trace", (registry, args: CommandArguments) => registry.trace(args)],
]);

const EMPTY_DIAGNOSTICS = Object.freeze({
  clear() {},
  set() {},
  delete() {},
  dispose() {},
});

export class CommandRegistry {
  declare api: HostApi;
  declare userInterface: UserInterface;
  declare options: EditorOptions;
  declare spawnProcess: EditorOptions["spawnProcess"];
  declare makeInvocationId: () => string;
  declare cliDiagnostics: Pick<
    import("vscode").DiagnosticCollection,
    "set" | "delete" | "clear" | "dispose"
  >;
  declare diagnosticUris: Map<string, import("vscode").Uri>;
  declare finiteGeneration: number;
  declare finiteSessions: Map<string, FiniteSession>;
  declare disposing: boolean;
  declare registered: boolean;
  declare watch: WatchCommand;

  constructor(api: HostApi, userInterface: UserInterface, options: EditorOptions = {}) {
    this.api = api;
    this.userInterface = userInterface;
    this.options = options;
    this.spawnProcess = options.spawnProcess;
    this.makeInvocationId = options.makeInvocationId ?? makeInvocationId;
    this.cliDiagnostics =
      api.languages?.createDiagnosticCollection?.("recite-cli") ?? EMPTY_DIAGNOSTICS;
    this.diagnosticUris = new Map();
    this.finiteGeneration = 0;
    this.finiteSessions = new Map();
    this.disposing = false;
    this.registered = false;
    this.watch = new WatchCommand(this);
  }

  register(subscriptions: import("vscode").Disposable[]) {
    if (this.registered || !this.api.commands?.registerCommand) return;
    this.registered = true;
    for (const [id, callback] of COMMANDS) {
      subscriptions.push(
        this.api.commands.registerCommand(id, (args: CommandArguments) => callback(this, args)),
      );
    }
  }

  async validate() {
    if (!this.trusted()) return undefined;
    let snapshot;
    try {
      snapshot = savedSourceSnapshot(this.userInterface);
    } catch (error) {
      this.failure(error);
      return undefined;
    }
    return this.finite("validate", [snapshot.path], { diagnostics: true }, undefined, snapshot);
  }

  async compile(args: CommandArguments = {}) {
    if (!this.trusted()) return undefined;
    let snapshot, configuration, output;
    try {
      snapshot = savedSourceSnapshot(this.userInterface);
      configuration = this.configuration();
      output = await requiredSavePath(
        args?.output,
        this.userInterface,
        this.api.Uri?.file?.(path.join(configuration.projectRoot, "dialogue.recitec")),
      );
    } catch (error) {
      this.failure(error);
      return undefined;
    }
    if (!output) return undefined;
    if (!this.revalidateSource(snapshot, configuration)) return undefined;
    return this.finite(
      "compile",
      ["--output", output, snapshot.path],
      { diagnostics: true },
      configuration,
      snapshot,
    );
  }

  async extract(args: CommandArguments = {}) {
    if (!this.trusted()) return undefined;
    let snapshot, configuration, output;
    try {
      snapshot = savedSourceSnapshot(this.userInterface);
      configuration = this.configuration();
      output = await optionalSavePath(
        args?.output,
        this.userInterface,
        this.api.Uri?.file?.(path.join(configuration.projectRoot, "messages.pot")),
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

  async run(args: CommandArguments = {}) {
    return this.runtime("run", args);
  }

  async trace(args: CommandArguments = {}) {
    return this.runtime("trace", args);
  }

  async runtime(command: string, args: CommandArguments) {
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
      command,
      "--output-format",
      "structured",
      "--invocation-id",
      invocationId,
      asset,
      "--block",
      block,
      "--fixture",
      fixture,
    ];
    return this.execute(configuration, command, invocationId, commandArgs, { diagnostics: false });
  }

  async watchStart(_args: CommandArguments = {}) {
    if (!this.trusted()) return undefined;
    let configuration;
    try {
      configuration = this.configuration();
    } catch (error) {
      this.failure(error);
      return undefined;
    }
    return this.watch.start(configuration);
  }

  async watchStop() {
    return this.watch.stop();
  }

  configuration() {
    return readCliConfiguration(this.api, this.userInterface);
  }

  configurationMatches(expected: CliConfiguration) {
    try {
      const current = this.configuration();
      return (
        current.command === expected.command &&
        current.cwd === expected.cwd &&
        current.projectRoot === expected.projectRoot &&
        current.projectRootOverridden === expected.projectRootOverridden
      );
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

  async finite(
    command: string,
    args: string[],
    options: { diagnostics: boolean },
    configurationOverride?: CliConfiguration,
    sourceSnapshot?: SourceSnapshot,
  ) {
    return finiteCommand(this, command, args, options, configurationOverride, sourceSnapshot);
  }

  async execute(
    configuration: CliConfiguration,
    command: string,
    invocationId: string,
    args: string[],
    options: { diagnostics: boolean },
  ) {
    return executeFiniteCommand(this, configuration, command, invocationId, args, options);
  }

  replaceFiniteDiagnostics(
    session: FiniteSession,
    records: unknown[],
    projectRoot: string | undefined,
  ) {
    return replaceFiniteDiagnostics(this, session, records, projectRoot);
  }

  replaceWatchDiagnostics(
    session: WatchSession,
    records: unknown[],
    projectRoot: string | undefined,
  ) {
    if (this.disposing || this.watch.active !== session) return;
    replaceDiagnostics(this.api, this.cliDiagnostics, records, projectRoot!, this.diagnosticUris);
  }

  clearDiagnostics() {
    clearDiagnostics(this.cliDiagnostics, this.diagnosticUris);
  }

  failure(error: unknown) {
    if (error instanceof CommandProtocolError) {
      const detail = error.code;
      this.userInterface.commandProtocolFailure(detail);
      return;
    }
    const detail = errorMessage(error);
    this.userInterface.commandFailure(detail);
  }

  revalidateSource(snapshot: SourceSnapshot, configuration: CliConfiguration) {
    try {
      if (!this.configurationMatches(configuration)) return false;
      assertSavedSource(this.userInterface, snapshot);
      const relative = path.relative(configuration.projectRoot, snapshot.path);
      if (
        !relative ||
        relative === ".." ||
        relative.startsWith(`..${path.sep}`) ||
        path.isAbsolute(relative)
      ) {
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
