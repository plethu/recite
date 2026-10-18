import type * as vscode from "vscode";
import type { SpawnOptionsWithoutStdio } from "node:child_process";
import type { EventEmitter } from "node:events";
import type { createUserInterface } from "./user-interface.ts";
import type { readConfiguration, readCliConfiguration } from "./configuration.ts";
import type { savedSourceSnapshot } from "./command-inputs.ts";
import type { runFiniteCommand, startStreamingCommand } from "./command-process.ts";
import type { WatchProtocolValidator } from "./watch-protocol.ts";

export type HostApi = typeof vscode & {
  Command?: new (title: string, command: string, ...args: unknown[]) => vscode.Command;
};
export type UserInterface = ReturnType<typeof createUserInterface>;
export type ServerConfiguration = ReturnType<typeof readConfiguration>;
export type CliConfiguration = ReturnType<typeof readCliConfiguration>;
export type SourceSnapshot = ReturnType<typeof savedSourceSnapshot>;
/** Only the process operations the transport owns; Node's ChildProcess fits this seam. */
export interface CommandChild {
  stdout: { on(event: "data", listener: (chunk: Buffer | string) => void): unknown };
  stderr: { on(event: "data", listener: (chunk: Buffer | string) => void): unknown };
  stdin: {
    writable: boolean;
    on(event: "error", listener: (error: Error) => void): unknown;
    write(value: string): unknown;
    end(): unknown;
    destroy(): unknown;
  };
  killed: boolean;
  kill(signal?: NodeJS.Signals): boolean;
  on(event: "error", listener: (error: Error) => void): unknown;
  on(event: "close", listener: (code: number | null, signal: string | null) => void): unknown;
}
export type SpawnProcess = (
  command: string,
  args: readonly string[],
  options: SpawnOptionsWithoutStdio,
) => CommandChild;
export type Timer = ReturnType<typeof setTimeout>;
export type ClientStatus = "idle" | "starting" | "running" | "stopped";
export interface StandardClient extends EventEmitter {
  status: ClientStatus;
  retired: boolean;
  failureReported?: boolean;
  start(): Promise<void>;
  stop(): Promise<void>;
  request(method: string, params: unknown, token?: vscode.CancellationToken): Promise<unknown>;
}
export interface EditorOptions {
  createClient?: (configuration: ServerConfiguration) => StandardClient;
  spawnProcess?: SpawnProcess;
  makeInvocationId?: () => string;
  stableRunMs?: number;
  restartDelaysMs?: readonly number[];
  editCommandTtlMs?: number;
  maxEditCommands?: number;
  retiredCommandTtlMs?: number;
  maxRetiredCommands?: number;
  maxStdoutBytes?: number;
  disposeTimeoutMs?: number;
  forceDisposeTimeoutMs?: number;
  authorityStopTimeoutMs?: number;
  authorityForceStopTimeoutMs?: number;
  watchStopTimeoutMs?: number;
  watchForceKillDelayMs?: number;
  watchTeardownTimeoutMs?: number;
  watchDisposeTimeoutMs?: number;
  getOpenDocument?: (uri: vscode.Uri) => vscode.TextDocument | undefined;
  clock?: { setTimeout?: typeof setTimeout; clearTimeout?: typeof clearTimeout };
}
export interface FiniteSession {
  generation: number;
  invocationId: string;
  child: CommandChild | undefined;
  promise: ReturnType<typeof runFiniteCommand> | undefined;
}
export interface WatchStopResult {
  stopped: true;
  exitCode: number | null;
}
export interface WatchSession {
  invocationId: string;
  configuration: CliConfiguration;
  validator: WatchProtocolValidator;
  projectRoot: string | undefined;
  stopped: boolean;
  retired: boolean;
  stopPromise: Promise<WatchStopResult | undefined> | undefined;
  stopResolve?: ((result: WatchStopResult | undefined) => void) | undefined;
  stopTimer: Timer | undefined;
  transport: ReturnType<typeof startStreamingCommand> | undefined;
  recovery: boolean;
  closed: boolean;
  forceTimer: Timer | undefined;
  teardownTimer: Timer | undefined;
  stopTimedOut: boolean;
}
export type VersionedEdit = vscode.WorkspaceEdit & {
  reciteVersionPreconditions?: Array<{
    document: vscode.TextDocument;
    version: number;
    uri: vscode.Uri;
  }>;
  reciteVersionStatus?: () =>
    | "current"
    | "document-closed"
    | "document-reopened"
    | "document-stale";
  reciteVersionGuard?: () => boolean;
};
export type EditFailure =
  | "document-stale"
  | "document-closed"
  | "document-reopened"
  | "expired"
  | "evicted"
  | "apply-failed"
  | "unknown";
export interface EditBatch {
  ids: Set<string>;
  active: boolean;
  finish(): void;
}
