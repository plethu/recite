import type { CommandRegistry } from "./commands.ts";
import type { Timer } from "./types.ts";
import { stopFiniteSessions } from "./finite-commands.ts";

export async function stopForAuthorityChange(registry: CommandRegistry) {
  await stopFiniteSessions(registry);
  if (!registry.watch.active) return undefined;
  return registry.watch.stopForAuthorityChange();
}

export async function disposeCommands(registry: CommandRegistry) {
  await registry.watch.dispose();
  const sessions = [...registry.finiteSessions.values()];
  for (const session of sessions) {
    try {
      session.child?.kill?.("SIGTERM");
    } catch {
      /* already gone */
    }
  }
  const settle = Promise.allSettled(sessions.map((session) => session.promise));
  await bounded(settle, registry.options.disposeTimeoutMs ?? 1_000);
  for (const session of sessions) {
    if (registry.finiteSessions.has(session.invocationId)) {
      try {
        session.child?.kill?.("SIGKILL");
      } catch {
        /* already gone */
      }
    }
  }
  await bounded(settle, registry.options.forceDisposeTimeoutMs ?? 250);
  registry.clearDiagnostics();
  registry.cliDiagnostics.dispose?.();
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
