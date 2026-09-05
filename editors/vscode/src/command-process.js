import { randomUUID } from "node:crypto";
import { spawn } from "node:child_process";
import { parseFiniteRecords } from "./finite-protocol.js";
import { protocol, NdjsonRecordParser, CommandProtocolError } from "./command-protocol.js";

const MAX_STDERR_BYTES = 4 * 1024 * 1024;

export class CommandProcessError extends Error {
  constructor(kind, detail, cause) {
    super(detail ?? kind, cause === undefined ? undefined : { cause });
    this.name = "ReciteCommandProcessError";
    this.kind = kind;
  }
}

export function makeInvocationId() {
  return randomUUID();
}

/** Run one finite structured command through an argv-only child process. */
export function runFiniteCommand(options) {
  const spawnProcess = options.spawnProcess ?? spawn;
  let child;
  try {
    child = spawnProcess(options.command, options.args, spawnOptions(options.cwd, options.environment));
    options.onSpawn?.(child);
  } catch (error) {
    return Promise.reject(processError("spawn", error));
  }
  const stdout = [];
  let stdoutBytes = 0;
  let stderrBytes = 0;
  let stderrSeen = false;
  let settled = false;

  return new Promise((resolve, reject) => {
    const fail = (error) => {
      if (settled) return;
      settled = true;
      terminateChild(child);
      const forceTimer = setTimeout(() => terminateChild(child, { force: true }), 100);
      forceTimer.unref?.();
      reject(error);
    };
    child.stdout.on("data", (chunk) => {
      if (settled) return;
      const buffer = Buffer.isBuffer(chunk) ? chunk : Buffer.from(String(chunk));
      stdoutBytes += buffer.byteLength;
      if (stdoutBytes > (options.maxStdoutBytes ?? 32 * 1024 * 1024)) {
        fail(protocol("stdout_too_large"));
        return;
      }
      stdout.push(buffer);
    });
    child.stderr.on("data", (chunk) => {
      if (settled) return;
      const bytes = Buffer.isBuffer(chunk) ? chunk.byteLength : Buffer.byteLength(String(chunk));
      if (bytes === 0) return;
      stderrSeen = true;
      stderrBytes += bytes;
      fail(stderrBytes > MAX_STDERR_BYTES
        ? protocol("stderr_too_large")
        : protocol("structured_stderr"));
    });
    child.stdin?.on?.("error", (error) => fail(processError("stdin", error)));
    child.on("error", (error) => fail(processError("process", error)));
    child.on("close", (code, signal) => {
      if (settled) return;
      settled = true;
      if (stderrSeen) {
        reject(protocol("structured_stderr"));
        return;
      }
      try {
        const result = parseFiniteRecords(Buffer.concat(stdout), options.commandName,
          options.invocationId, code);
        resolve({ ...result, exitCode: code, signal });
      } catch (error) {
        reject(error);
      }
    });
    endStdin(child);
  });
}

/**
 * Start a long-lived structured process. The caller owns protocol state and
 * decides when a valid terminal record has arrived; this seam only owns argv,
 * bounded records, stderr refusal, and child teardown.
 */
export function startStreamingCommand(options) {
  const spawnProcess = options.spawnProcess ?? spawn;
  let child;
  try {
    child = spawnProcess(options.command, options.args, spawnOptions(options.cwd, options.environment));
  } catch (error) {
    throw processError("spawn", error);
  }
  const parser = new NdjsonRecordParser();
  let stderrBytes = 0;
  let closed = false;
  let failed = false;
  const fail = (error) => {
    if (failed || closed) return;
    failed = true;
    options.onError?.(error);
    terminateChild(child);
  };
  child.stdout.on("data", (chunk) => {
    if (failed || closed) return;
    try {
      for (const record of parser.push(chunk)) options.onRecord?.(record);
    } catch (error) {
      fail(error instanceof CommandProtocolError ? error : protocol("stdout_protocol", error.message));
    }
  });
  child.stderr.on("data", (chunk) => {
    if (failed || closed) return;
    stderrBytes += Buffer.isBuffer(chunk) ? chunk.byteLength : Buffer.byteLength(String(chunk));
    if (stderrBytes > MAX_STDERR_BYTES || stderrBytes > 0) fail(protocol("structured_stderr"));
  });
  child.stdin?.on?.("error", (error) => fail(processError("stdin", error)));
  child.on("error", (error) => fail(processError("process", error)));
  child.on("close", (code, signal) => {
    if (closed) return;
    closed = true;
    if (!failed) {
      try {
        parser.finish();
        options.onClose?.({ code, signal, failed: false });
      } catch (error) {
        options.onError?.(error);
        options.onClose?.({ code, signal, failed: true });
      }
    } else {
      options.onClose?.({ code, signal, failed: true });
    }
  });
  return {
    child,
    parser,
    write(value) {
      if (closed || failed || !child.stdin?.writable) return false;
      try {
        child.stdin.write(`${JSON.stringify(value)}\n`);
        return true;
      } catch (error) {
        fail(processError("stdin", error));
        return false;
      }
    },
    terminate() {
      terminateChild(child);
    },
    forceTerminate() {
      terminateChild(child, { force: true });
    }
  };
}

function spawnOptions(cwd, environment) {
  return {
    cwd,
    env: { ...process.env, ...(environment ?? {}) },
    shell: false,
    stdio: ["pipe", "pipe", "pipe"]
  };
}

function processError(kind, error) {
  return error instanceof CommandProcessError
    ? error
    : new CommandProcessError(kind, error?.message ?? String(error), error);
}

function endStdin(child) {
  try { child.stdin?.end?.(); } catch { /* a child may close stdin before startup */ }
}

export function terminateChild(child, { force = false } = {}) {
  try { child.stdin?.destroy?.(); } catch { /* already closed */ }
  if (child && (!child.killed || force)) {
    try { child.kill?.(force ? "SIGKILL" : "SIGTERM"); } catch { /* already gone */ }
  }
}
