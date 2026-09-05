import { startStreamingCommand } from "./command-process.js";
import { WatchProtocolValidator } from "./watch-protocol.js";
import { protocol } from "./command-protocol.js";

const STOP_TIMEOUT_MS = 1_500;
const FORCE_KILL_DELAY_MS = 100;

/** Owns the one active structured watch child and its generation identity. */
export class WatchCommand {
  constructor(registry) {
    this.registry = registry;
    this.active = undefined;
    this.stopTimeoutMs = registry.options.watchStopTimeoutMs ?? STOP_TIMEOUT_MS;
    this.forceKillDelayMs = registry.options.watchForceKillDelayMs ?? FORCE_KILL_DELAY_MS;
    this.clock = registry.options.clock ?? {};
  }

  start(configuration) {
    if (this.active) {
      this.registry.userInterface.commandWatchRunning();
      return Promise.resolve(undefined);
    }
    const invocationId = this.registry.makeInvocationId();
    const session = {
      invocationId,
      configuration,
      validator: new WatchProtocolValidator("watch", invocationId, configuration.projectRoot),
      projectRoot: undefined,
      stopped: false,
      retired: false,
      stopPromise: undefined,
      stopTimer: undefined,
      transport: undefined,
      recovery: false,
      closed: false,
      forceTimer: undefined,
      teardownTimer: undefined,
      stopTimedOut: false
    };
    this.active = session;
    try {
      session.transport = startStreamingCommand({
        command: configuration.command,
        args: ["watch", "--output-format", "structured", "--invocation-id", invocationId,
          configuration.projectRoot],
        cwd: configuration.cwd,
        invocationId,
        spawnProcess: this.registry.spawnProcess,
        onRecord: (record) => this.record(session, record),
        onError: (error) => this.error(session, error),
        onClose: (event) => this.close(session, event)
      });
    } catch (error) {
      this.error(session, error);
    }
    return Promise.resolve(this.active === session ? { invocationId } : undefined);
  }

  stop() {
    const session = this.active;
    if (!session) {
      this.registry.userInterface.commandWatchNotRunning();
      return Promise.resolve(undefined);
    }
    if (session.stopPromise) return session.stopPromise;
    session.stopPromise = new Promise((resolve) => {
      session.stopResolve = resolve;
    });
    if (!session.transport?.write({
      version: 1,
      command: "watch",
      action: "cancel",
      invocation_id: session.invocationId
    })) {
      this.error(session, protocol("cancel_write_failed"));
      return session.stopPromise;
    }
    const setTimeout_ = this.clock.setTimeout ?? setTimeout;
    session.stopTimer = setTimeout_(() => {
      // A stopped record is only the first half of a cooperative stop. Keep
      // ownership until the process exit arrives; a hung child still needs
      // the bounded recovery path below.
      if (this.active !== session) return;
      session.stopTimedOut = true;
      this.registry.userInterface.commandWatchStopTimeout();
      this.recover(session, false);
    }, this.stopTimeoutMs);
    session.stopTimer?.unref?.();
    return session.stopPromise;
  }

  stopForAuthorityChange() {
    if (!this.active) return Promise.resolve(undefined);
    return this.stop();
  }

  record(session, record) {
    if (this.active !== session || session.retired || session.recovery) return;
    try {
      session.validator.consume(record);
      if (record.event === "watch.started") session.projectRoot = session.validator.projectRoot;
      if (["watch.started", "watch.build.started", "watch.waiting", "watch.cancel.requested",
        "watch.control.error", "watch.notify.error"].includes(record.event)) {
        const detail = JSON.stringify(record.data);
        this.registry.userInterface.commandWatchStatus(detail);
      } else if (record.event === "watch.build.completed") {
        // A completion is a replacement snapshot. Clearing before applying
        // it removes diagnostics for files that disappeared from this build.
        // Watch spans are disk-backed; dirty editor overlays remain owned by
        // the LSP until a later saved build is reported.
        this.registry.replaceWatchDiagnostics(
          session,
          record.data.diagnostics,
          session.projectRoot
        );
        const detail = JSON.stringify(record.data);
        this.registry.userInterface.commandWatchStatus(detail);
      } else if (record.event === "watch.stopped") {
        session.stopped = true;
        if (record.data.reason.type === "fatal") {
          const detail = JSON.stringify(record.data.error);
          this.registry.userInterface.commandFailure(detail);
          // A fatal terminal is semantically complete, but it does not prove
          // that the child has exited. Reuse the bounded recovery path so a
          // broken CLI cannot retain ownership forever or overlap a restart.
          this.recover(session);
        } else {
          const detail = JSON.stringify(record.data);
          this.registry.userInterface.commandWatchStatus(detail);
        }
      }
    } catch (error) {
      this.error(session, error);
    }
  }

  error(session, error) {
    if (this.active !== session || session.retired) return;
    this.registry.failure(error);
    if (!session.transport) {
      this.retire(session);
      this.resolveStop(session, undefined);
      return;
    }
    this.recover(session, true);
  }

  close(session, { code, failed = false } = {}) {
    if (this.active !== session || session.retired) return;
    session.closed = true;
    if (failed || session.recovery) {
      this.retire(session);
      this.resolveStop(session, undefined);
      return;
    }
    try {
      session.validator.finish(code);
      this.retire(session);
      this.resolveStop(session, { stopped: true, exitCode: code });
    } catch (error) {
      this.registry.failure(error);
      this.retire(session);
      this.resolveStop(session, undefined);
    }
  }

  recover(session) {
    if (this.active !== session || session.retired || session.recovery) return;
    session.recovery = true;
    session.transport?.terminate();
    const setTimeout_ = this.clock.setTimeout ?? setTimeout;
    session.forceTimer = setTimeout_(() => {
      if (this.active !== session || session.retired || session.closed) return;
      session.transport?.forceTerminate?.();
      if (this.active !== session || session.retired || session.closed) return;
      session.teardownTimer = setTimeout_(() => {
        if (this.active !== session || session.retired || session.closed) return;
        // Keep this tombstone owned until the child reports close. The public
        // stop operation is bounded, while a later close still retires it.
        this.resolveStop(session, undefined);
      }, this.registry.options.watchTeardownTimeoutMs ?? this.forceKillDelayMs + 400);
      session.teardownTimer?.unref?.();
    }, this.forceKillDelayMs);
    session.forceTimer?.unref?.();
  }

  retire(session) {
    if (session.retired) return;
    session.retired = true;
    if (session.stopTimer !== undefined) {
      (this.clock.clearTimeout ?? clearTimeout)(session.stopTimer);
      session.stopTimer = undefined;
    }
    if (session.forceTimer !== undefined) {
      (this.clock.clearTimeout ?? clearTimeout)(session.forceTimer);
      session.forceTimer = undefined;
    }
    if (session.teardownTimer !== undefined) {
      (this.clock.clearTimeout ?? clearTimeout)(session.teardownTimer);
      session.teardownTimer = undefined;
    }
    if (this.active === session) this.active = undefined;
    this.registry.clearDiagnostics();
  }

  resolveStop(session, result) {
    if (!session.stopResolve) return;
    const resolve = session.stopResolve;
    session.stopResolve = undefined;
    resolve(result);
  }

  async dispose() {
    const session = this.active;
    if (!session) return;
    const stopping = this.stop();
    const timeout = new Promise((resolve) => {
      const setTimeout_ = this.clock.setTimeout ?? setTimeout;
      const timer = setTimeout_(resolve, this.registry.options.watchDisposeTimeoutMs ??
        this.stopTimeoutMs + this.forceKillDelayMs + 100);
    });
    await Promise.race([stopping, timeout]);
    if (this.active === session && !session.recovery) this.recover(session, false);
  }
}
