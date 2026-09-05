import { assertSavedSource } from "./command-inputs.js";
import { runFiniteCommand } from "./command-process.js";
import { CommandProtocolError } from "./command-protocol.js";
import { replaceDiagnostics } from "./command-diagnostics.js";

export async function finiteCommand(registry, command, args, options, configurationOverride,
  sourceSnapshot) {
  if (!registry.trusted()) return undefined;
  let configuration = configurationOverride;
  try {
    if (!configuration) configuration = registry.configuration();
    if (sourceSnapshot && !registry.revalidateSource(sourceSnapshot, configuration)) return undefined;
  } catch (error) {
    registry.failure(error);
    return undefined;
  }
  const invocationId = registry.makeInvocationId();
  const commandArgs = [command, ...args, "--output-format", "structured", "--invocation-id", invocationId];
  return executeFiniteCommand(registry, configuration, command, invocationId, commandArgs,
    { ...options, sourceSnapshot });
}

export async function executeFiniteCommand(registry, configuration, command, invocationId, args,
  { diagnostics, sourceSnapshot }) {
  if (!registry.trusted(false)) return undefined;
  if (!registry.configurationMatches(configuration)) return undefined;
  if (sourceSnapshot) {
    try { assertSavedSource(registry.userInterface, sourceSnapshot); } catch (error) {
      registry.failure(error);
      return undefined;
    }
  }
  const generation = ++registry.finiteGeneration;
  const session = { generation, invocationId, child: undefined, promise: undefined };
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
      onSpawn: (child) => { session.child = child; }
    });
    session.promise = promise;
    const result = await promise;
    if (sourceSnapshot) {
      try { assertSavedSource(registry.userInterface, sourceSnapshot); } catch (error) {
        if (!registry.disposing && session.generation === registry.finiteGeneration) registry.failure(error);
        return undefined;
      }
    }
    if (session.generation !== registry.finiteGeneration || registry.disposing) return undefined;
    if (result.terminal.event === "command.error") {
      registry.userInterface.commandFailure(JSON.stringify(result.terminal.error));
      return result;
    }
    if (diagnostics) replaceFiniteDiagnostics(registry, session, result.terminal.data.diagnostics,
      configuration.projectRoot);
    const detail = JSON.stringify(result.terminal.data);
    if (result.terminal.status === "content_diagnostics") {
      registry.userInterface.commandContentDiagnostics(detail);
    } else {
      registry.userInterface.commandResult(detail);
    }
    return result;
  } catch (error) {
    if (!registry.disposing && session.generation === registry.finiteGeneration) registry.failure(error);
    return undefined;
  } finally {
    registry.finiteSessions.delete(invocationId);
  }
}

export function replaceFiniteDiagnostics(registry, session, records, projectRoot) {
  if (registry.disposing || session.generation !== registry.finiteGeneration || registry.watch.active) return;
  replaceDiagnostics(registry.api, registry.cliDiagnostics, records, projectRoot, registry.diagnosticUris);
}

export async function stopFiniteSessions(registry) {
  const sessions = [...registry.finiteSessions.values()];
  if (sessions.length === 0) return;
  registry.finiteGeneration += 1;
  for (const session of sessions) {
    try { session.child?.kill?.("SIGTERM"); } catch { /* already gone */ }
  }
  const settle = Promise.allSettled(sessions.map((session) => session.promise));
  await bounded(settle, registry.options.authorityStopTimeoutMs ?? 500);
  for (const session of sessions) {
    if (registry.finiteSessions.has(session.invocationId)) {
      try { session.child?.kill?.("SIGKILL"); } catch { /* already gone */ }
    }
  }
  await bounded(settle, registry.options.authorityForceStopTimeoutMs ?? 250);
}

async function bounded(promise, milliseconds) {
  let timer;
  await Promise.race([promise, new Promise((resolve) => {
    timer = setTimeout(resolve, milliseconds);
  })]);
  if (timer) clearTimeout(timer);
}
