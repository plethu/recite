import type { CommandRegistry } from "./commands.ts";
import type { CliConfiguration, SourceSnapshot, FiniteSession, Timer } from "./types.ts";
import { replaceDiagnostics } from "./command-diagnostics.ts";
import { assertSavedSource } from "./command-inputs.ts";
import { runFiniteCommand } from "./command-process.ts";

export async function finiteCommand(
  registry: CommandRegistry,
  command: string,
  args: string[],
  options: { diagnostics: boolean },
  configurationOverride?: CliConfiguration,
  sourceSnapshot?: SourceSnapshot,
) {
  if (!registry.trusted()) return undefined;
  let configuration = configurationOverride;
  try {
    if (!configuration) configuration = registry.configuration();
    if (sourceSnapshot && !registry.revalidateSource(sourceSnapshot, configuration)) {
      return undefined;
    }
  } catch (error) {
    registry.failure(error);
    return undefined;
  }
  const invocationId = registry.makeInvocationId();
  const commandArgs = [
    command,
    ...args,
    "--output-format",
    "structured",
    "--invocation-id",
    invocationId,
  ];
  return executeFiniteCommand(registry, configuration, command, invocationId, commandArgs, {
    ...options,
    sourceSnapshot,
  });
}

export async function executeFiniteCommand(
  registry: CommandRegistry,
  configuration: CliConfiguration,
  command: string,
  invocationId: string,
  args: string[],
  {
    diagnostics,
    sourceSnapshot,
  }: { diagnostics: boolean; sourceSnapshot?: SourceSnapshot | undefined },
) {
  if (!registry.trusted(false)) return undefined;
  if (!registry.configurationMatches(configuration)) return undefined;
  if (sourceSnapshot) {
    try {
      assertSavedSource(registry.userInterface, sourceSnapshot);
    } catch (error) {
      registry.failure(error);
      return undefined;
    }
  }
  const generation = ++registry.finiteGeneration;
  const session: FiniteSession = { generation, invocationId, child: undefined, promise: undefined };
  registry.finiteSessions.set(invocationId, session);
  try {
    const promise = runFiniteCommand({
      command: configuration.command,
      commandName: command,
      args,
      cwd: configuration.cwd,
      invocationId,
      spawnProcess: registry.spawnProcess,
      maxStdoutBytes: registry.options.maxStdoutBytes ?? 32 * 1024 * 1024,
      onSpawn: (child) => {
        session.child = child;
      },
    });
    session.promise = promise;
    const result = await promise;
    if (sourceSnapshot) {
      try {
        assertSavedSource(registry.userInterface, sourceSnapshot);
      } catch (error) {
        if (!registry.disposing && session.generation === registry.finiteGeneration) {
          registry.failure(error);
        }
        return undefined;
      }
    }
    if (session.generation !== registry.finiteGeneration || registry.disposing) return undefined;
    if (result.terminal.event === "command.error") {
      registry.userInterface.commandFailure(JSON.stringify(result.terminal.error));
      return result;
    }
    if (diagnostics) {
      replaceFiniteDiagnostics(
        registry,
        session,
        "diagnostics" in result.terminal.data ? result.terminal.data.diagnostics : [],
        configuration.projectRoot,
      );
    }
    const detail = JSON.stringify(result.terminal.data);
    if (result.terminal.status === "content_diagnostics") {
      registry.userInterface.commandContentDiagnostics(detail);
    } else {
      registry.userInterface.commandResult(detail);
    }
    return result;
  } catch (error) {
    if (!registry.disposing && session.generation === registry.finiteGeneration) {
      registry.failure(error);
    }
    return undefined;
  } finally {
    registry.finiteSessions.delete(invocationId);
  }
}

export function replaceFiniteDiagnostics(
  registry: CommandRegistry,
  session: FiniteSession,
  records: unknown,
  projectRoot: string | undefined,
) {
  if (
    registry.disposing ||
    session.generation !== registry.finiteGeneration ||
    registry.watch.active
  )
    return;
  replaceDiagnostics(
    registry.api,
    registry.cliDiagnostics,
    records,
    projectRoot!,
    registry.diagnosticUris,
  );
}

export async function stopFiniteSessions(registry: CommandRegistry) {
  const sessions = [...registry.finiteSessions.values()];
  if (sessions.length === 0) return;
  registry.finiteGeneration += 1;
  for (const session of sessions) {
    try {
      session.child?.kill?.("SIGTERM");
    } catch {
      /* already gone */
    }
  }
  const settle = Promise.allSettled(sessions.map((session) => session.promise));
  await bounded(settle, registry.options.authorityStopTimeoutMs ?? 500);
  for (const session of sessions) {
    if (registry.finiteSessions.has(session.invocationId)) {
      try {
        session.child?.kill?.("SIGKILL");
      } catch {
        /* already gone */
      }
    }
  }
  await bounded(settle, registry.options.authorityForceStopTimeoutMs ?? 250);
}

async function bounded(promise: Promise<unknown>, milliseconds: number) {
  let timer: Timer | undefined;
  await Promise.race([
    promise,
    new Promise<void>((resolve) => {
      timer = setTimeout(resolve, milliseconds);
    }),
  ]);
  if (timer) clearTimeout(timer);
}
