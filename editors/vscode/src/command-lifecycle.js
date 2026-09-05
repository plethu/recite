import { stopFiniteSessions } from "./finite-commands.js";

export async function stopForAuthorityChange(registry) {
  await stopFiniteSessions(registry);
  if (!registry.watch.active) return undefined;
  return registry.watch.stopForAuthorityChange();
}

export async function disposeCommands(registry) {
  await registry.watch.dispose();
  const sessions = [...registry.finiteSessions.values()];
  for (const session of sessions) {
    try { session.child?.kill?.("SIGTERM"); } catch { /* already gone */ }
  }
  const settle = Promise.allSettled(sessions.map((session) => session.promise));
  await bounded(settle, registry.options.disposeTimeoutMs ?? 1_000);
  for (const session of sessions) {
    if (registry.finiteSessions.has(session.invocationId)) {
      try { session.child?.kill?.("SIGKILL"); } catch { /* already gone */ }
    }
  }
  await bounded(settle, registry.options.forceDisposeTimeoutMs ?? 250);
  registry.clearDiagnostics();
  registry.cliDiagnostics.dispose?.();
}

async function bounded(promise, milliseconds) {
  let timer;
  await Promise.race([promise, new Promise((resolve) => {
    timer = setTimeout(resolve, milliseconds);
  })]);
  if (timer) clearTimeout(timer);
}
