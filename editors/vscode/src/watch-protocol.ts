import type { WatchRecord, MachinePath } from "./watch-types.ts";
import type { IntegerValue, JsonRecord } from "./command-protocol.ts";
import { errorMessage } from "./errors.ts";
import { realpathSync } from "node:fs";
import * as path from "node:path";
import {
  compareIntegers,
  exactEnvelopeKeys,
  protocol,
  validateEnvelope,
} from "./command-protocol.ts";
import { is, watchRecordSchema } from "./schemas/index.js";
const eventErrors = new Map([
  ["watch.started", "invalid_watch_started"],
  ["watch.build.started", "invalid_build_started"],
  ["watch.build.completed", "invalid_build_completed"],
  ["watch.waiting", "invalid_watch_waiting"],
  ["watch.cancel.requested", "invalid_cancel"],
  ["watch.control.error", "invalid_control_error"],
  ["watch.notify.error", "invalid_notify_error"],
  ["watch.stopped", "invalid_watch_stopped"],
]);

export class WatchProtocolValidator {
  declare command: string;
  declare invocationId: string | undefined;
  declare sequence: number;
  declare started: boolean;
  declare activeGeneration: IntegerValue | undefined;
  declare lastGeneration: IntegerValue | undefined;
  declare phase:
    | "before_started"
    | "awaiting_build"
    | "building"
    | "awaiting_wait"
    | "stopped_ready"
    | "stopped";
  declare stopped: boolean;
  declare stopReason: "cancelled" | "fatal" | undefined;
  declare cancelRequested: boolean;
  declare projectRoot: string | undefined;
  declare expectedProjectRoot: string | undefined;

  constructor(
    command: string,
    invocationId: string | undefined,
    expectedProjectRoot: string | undefined,
  ) {
    this.command = command;
    this.invocationId = invocationId;
    this.sequence = 0;
    this.started = false;
    this.activeGeneration = undefined;
    this.lastGeneration = undefined;
    this.phase = "before_started";
    this.stopped = false;
    this.stopReason = undefined;
    this.cancelRequested = false;
    this.projectRoot = undefined;
    this.expectedProjectRoot = expectedProjectRoot;
  }

  consume(input: JsonRecord): WatchRecord {
    const record = input;

    if (this.stopped) throw protocol("records_after_stopped");
    validateEnvelope(record, this.command, this.invocationId, this.sequence++);
    if (
      !exactEnvelopeKeys(
        record,
        ["version", "sequence", "event", "command", "invocation_id", "data"],
        this.invocationId,
      )
    ) {
      throw protocol("invalid_envelope");
    }
    if (!is(watchRecordSchema, record)) {
      throw protocol(
        typeof record.event === "string"
          ? (eventErrors.get(record.event) ?? "unknown_watch_event")
          : "unknown_watch_event",
      );
    }
    switch (record.event) {
      case "watch.started":
        this.startedRecord(record);
        break;
      case "watch.build.started":
        this.buildStarted(record);
        break;
      case "watch.build.completed":
        this.buildCompleted(record);
        break;
      case "watch.waiting":
        if (!this.started || this.phase !== "awaiting_wait") {
          throw protocol("invalid_watch_waiting");
        }
        this.phase = "awaiting_build";
        break;
      case "watch.cancel.requested":
        this.cancel(record);
        break;
      case "watch.control.error":
        if (!this.started) throw protocol("invalid_control_error");
        break;
      case "watch.notify.error":
        if (!this.started) throw protocol("invalid_notify_error");
        break;
      case "watch.stopped":
        this.stoppedRecord(record);
        break;
      default:
        throw protocol("unknown_watch_event");
    }
    return record;
  }

  finish(exitCode: number | null) {
    if (!this.stopped || this.activeGeneration !== undefined) throw protocol("watch_not_stopped");
    if (!Number.isInteger(exitCode)) throw protocol("missing_process_exit");
    const expected = this.stopReason === "cancelled" ? 0 : 1;
    if (exitCode !== expected) throw protocol("watch_exit_mismatch");
  }

  startedRecord(record: Extract<WatchRecord, { event: "watch.started" }>) {
    if (this.started) {
      throw protocol("invalid_watch_started");
    }
    this.projectRoot = decodeMachinePath(record.data.project_root);
    if (!this.projectRoot || !path.isAbsolute(this.projectRoot)) {
      throw protocol("invalid_watch_project_root");
    }
    this.projectRoot = path.normalize(this.projectRoot);
    if (
      this.expectedProjectRoot !== undefined &&
      !sameProjectRoot(this.projectRoot, this.expectedProjectRoot)
    ) {
      throw protocol("watch_project_root_mismatch");
    }
    this.started = true;
    this.phase = "awaiting_build";
  }

  buildStarted(record: Extract<WatchRecord, { event: "watch.build.started" }>) {
    const expectedTrigger = this.lastGeneration === undefined ? "initial" : "input_changed";
    if (
      !this.started ||
      this.cancelRequested ||
      this.phase !== "awaiting_build" ||
      record.data.trigger !== expectedTrigger ||
      (this.lastGeneration === undefined && record.data.generation !== 0) ||
      (this.lastGeneration !== undefined &&
        compareIntegers(record.data.generation, this.lastGeneration)! <= 0)
    ) {
      throw protocol("invalid_build_started");
    }
    this.activeGeneration = record.data.generation;
    this.lastGeneration = record.data.generation;
    this.phase = "building";
  }

  buildCompleted(record: Extract<WatchRecord, { event: "watch.build.completed" }>) {
    if (
      !this.started ||
      this.phase !== "building" ||
      this.activeGeneration === undefined ||
      compareIntegers(record.data.generation, this.activeGeneration) !== 0
    ) {
      throw protocol("invalid_build_completed");
    }
    this.activeGeneration = undefined;
    this.phase = this.cancelRequested ? "stopped_ready" : "awaiting_wait";
  }

  cancel(record: Extract<WatchRecord, { event: "watch.cancel.requested" }>) {
    if (
      !this.started ||
      this.cancelRequested ||
      record.data?.cancellation?.type !== "user" ||
      this.stopReason ||
      !["awaiting_build", "building"].includes(this.phase)
    ) {
      throw protocol("invalid_cancel");
    }
    this.cancelRequested = true;
    if (this.phase === "awaiting_build") this.phase = "stopped_ready";
  }

  stoppedRecord(record: Extract<WatchRecord, { event: "watch.stopped" }>) {
    const phaseAllowsStop =
      ["awaiting_build", "stopped_ready"].includes(this.phase) ||
      (this.phase === "awaiting_wait" && record.data?.reason?.type === "fatal");
    if (!this.started || this.stopReason || !phaseAllowsStop)
      throw protocol("invalid_watch_stopped");
    if (record.data.reason.type === "cancelled" && !this.cancelRequested) {
      throw protocol("invalid_watch_stopped");
    }
    this.stopReason = record.data.reason.type;
    this.stopped = true;
    this.phase = "stopped";
  }
}

function decodeMachinePath(value: MachinePath) {
  try {
    if (value.encoding === "utf8") return value.value;
    if (value.encoding === "unix_bytes") {
      return new TextDecoder("utf-8", { fatal: true }).decode(Buffer.from(value.value, "hex"));
    }
    let decoded = "";
    for (let index = 0; index < value.value.length; index += 4096) {
      decoded += String.fromCharCode(...value.value.slice(index, index + 4096));
    }
    return decoded;
  } catch (error) {
    throw protocol("invalid_machine_path", errorMessage(error));
  }
}

function sameProjectRoot(actual: string, expected: string) {
  if (typeof expected !== "string" || !path.isAbsolute(expected)) return false;
  const normalizedExpected = path.normalize(expected);
  if (actual === normalizedExpected) return true;
  try {
    return realpathSync.native(actual) === realpathSync.native(normalizedExpected);
  } catch {
    // A missing or non-representable host path cannot be proven equivalent.
    // Keep the protocol fail-closed instead of widening diagnostic authority.
    return false;
  }
}
