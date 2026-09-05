import * as path from "node:path";
import { realpathSync } from "node:fs";
import { compareIntegers, exactEnvelopeKeys, protocol, validateEnvelope } from "./command-protocol.js";
import {
  validBuildCompleted,
  validBuildStart,
  validCancellation,
  validStop
} from "./watch-record-validation.js";

export class WatchProtocolValidator {
  constructor(command, invocationId, expectedProjectRoot) {
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

  consume(record) {
    if (this.stopped) throw protocol("records_after_stopped");
    validateEnvelope(record, this.command, this.invocationId, this.sequence++);
    if (!exactEnvelopeKeys(record, ["version", "sequence", "event", "command", "invocation_id", "data"], this.invocationId)) {
      throw protocol("invalid_envelope");
    }
    switch (record.event) {
      case "watch.started": this.startedRecord(record); break;
      case "watch.build.started": this.buildStarted(record); break;
      case "watch.build.completed": this.buildCompleted(record); break;
      case "watch.waiting":
        if (!this.started || this.phase !== "awaiting_wait" || !emptyData(record.data)) {
          throw protocol("invalid_watch_waiting");
        }
        this.phase = "awaiting_build";
        break;
      case "watch.cancel.requested": this.cancel(record); break;
      case "watch.control.error":
        if (!this.started || !dataKeys(record.data, ["error"]) || !exactKeys(record.data.error, ["type"]) || !controlErrors.has(record.data?.error?.type)) throw protocol("invalid_control_error");
        break;
      case "watch.notify.error":
        if (!this.started || !dataKeys(record.data, ["error"]) || !exactKeys(record.data.error, ["type"]) || record.data?.error?.type !== "watcher") throw protocol("invalid_notify_error");
        break;
      case "watch.stopped": this.stoppedRecord(record); break;
      default: throw protocol("unknown_watch_event");
    }
    return record;
  }

  finish(exitCode) {
    if (!this.stopped || this.activeGeneration !== undefined) throw protocol("watch_not_stopped");
    if (!Number.isInteger(exitCode)) throw protocol("missing_process_exit");
    const expected = this.stopReason === "cancelled" ? 0 : 1;
    if (exitCode !== expected) throw protocol("watch_exit_mismatch");
  }

  startedRecord(record) {
    if (this.started || !dataKeys(record.data, ["project_root"]) || !machinePath(record.data.project_root)) {
      throw protocol("invalid_watch_started");
    }
    this.projectRoot = decodeMachinePath(record.data.project_root);
    if (!this.projectRoot || !path.isAbsolute(this.projectRoot)) throw protocol("invalid_watch_project_root");
    this.projectRoot = path.normalize(this.projectRoot);
    if (this.expectedProjectRoot !== undefined &&
        !sameProjectRoot(this.projectRoot, this.expectedProjectRoot)) {
      throw protocol("watch_project_root_mismatch");
    }
    this.started = true;
    this.phase = "awaiting_build";
  }

  buildStarted(record) {
    const expectedTrigger = this.lastGeneration === undefined ? "initial" : "input_changed";
    if (!this.started || this.cancelRequested || this.phase !== "awaiting_build" ||
        !dataKeys(record.data, ["generation", "trigger"]) || !validBuildStart(record.data) ||
        record.data.trigger !== expectedTrigger ||
        this.lastGeneration === undefined && record.data.generation !== 0 ||
        this.lastGeneration !== undefined && compareIntegers(record.data.generation, this.lastGeneration) <= 0) {
      throw protocol("invalid_build_started");
    }
    this.activeGeneration = record.data.generation;
    this.lastGeneration = record.data.generation;
    this.phase = "building";
  }

  buildCompleted(record) {
    if (!this.started || this.phase !== "building" || this.activeGeneration === undefined ||
        !dataKeys(record.data, ["generation", "snapshot_generation", "status", "outcome", "inputs",
          "diagnostics", "artifacts", "freshness", "publication", "recovery", "restart_guidance"],
          ["cancellation", "failure", "error"]) ||
        !validBuildCompleted(record.data) || compareIntegers(record.data.generation, this.activeGeneration) !== 0) {
      throw protocol("invalid_build_completed");
    }
    this.activeGeneration = undefined;
    this.phase = this.cancelRequested ? "stopped_ready" : "awaiting_wait";
  }

  cancel(record) {
    if (!this.started || this.cancelRequested || record.data?.cancellation?.type !== "user" ||
        !dataKeys(record.data, ["cancellation"]) || !validCancellation(record.data?.cancellation) ||
        this.stopReason || !["awaiting_build", "building"].includes(this.phase)) {
      throw protocol("invalid_cancel");
    }
    this.cancelRequested = true;
    if (this.phase === "awaiting_build") this.phase = "stopped_ready";
  }

  stoppedRecord(record) {
    if (!this.started || this.stopReason || !dataKeys(record.data, ["reason"], ["error"]) || !validStop(record.data) ||
        !["awaiting_build", "stopped_ready"].includes(this.phase)) throw protocol("invalid_watch_stopped");
    if (record.data.reason.type === "cancelled" && !this.cancelRequested) throw protocol("invalid_watch_stopped");
    this.stopReason = record.data.reason.type;
    this.stopped = true;
    this.phase = "stopped";
  }
}

function machinePath(value) {
  return Boolean(value && typeof value === "object" && exactKeys(value, ["encoding", "value"]) &&
    ((value.encoding === "utf8" && typeof value.value === "string") ||
      (value.encoding === "unix_bytes" && typeof value.value === "string" &&
        /^[0-9a-f]*$/u.test(value.value) && value.value.length % 2 === 0) ||
      (value.encoding === "windows_wtf16" && Array.isArray(value.value) &&
        value.value.every((unit) => Number.isSafeInteger(unit) && unit >= 0 && unit <= 0xffff))));
}

function decodeMachinePath(value) {
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
    throw protocol("invalid_machine_path", error.message);
  }
}

function sameProjectRoot(actual, expected) {
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

function emptyData(value) { return Boolean(value && typeof value === "object" && !Array.isArray(value) && Object.keys(value).length === 0); }
function dataKeys(value, required, optional = []) {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const allowed = new Set([...required, ...optional]);
  return required.every((key) => Object.hasOwn(value, key)) && Object.keys(value).every((key) => allowed.has(key));
}

const controlErrors = new Set(["malformed", "unsupported_version", "unsupported_command", "unsupported_action", "invocation_mismatch"]);

function exactKeys(value, expected) {
  return value && typeof value === "object" && JSON.stringify(Object.keys(value).sort()) ===
    JSON.stringify(expected.slice().sort());
}
