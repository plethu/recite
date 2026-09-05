import { NdjsonRecordParser, integerInRange, protocol, validateEnvelope } from "./command-protocol.js";
import { validDiagnosticRecord } from "./command-diagnostics.js";
import { validStructuredError } from "./watch-record-validation.js";
import { validTrace } from "./trace-record-validation.js";

const MAX_FINITE_BYTES = 32 * 1024 * 1024;

export function parseFiniteRecords(stdout, command, invocationId, exitCode) {
  if (!["validate", "compile", "extract", "run", "trace"].includes(command)) {
    throw protocol("unsupported_command");
  }
  const parser = new NdjsonRecordParser({ maxBytes: MAX_FINITE_BYTES });
  const records = parser.push(stdout);
  parser.finish();
  if (records.length !== 2) throw protocol("finite_record_count");
  validateEnvelope(records[0], command, invocationId, 0);
  validateEnvelope(records[1], command, invocationId, 1);
  if (!exactKeys(records[0], ["version", "sequence", "event", "command", "invocation_id"]) ||
      records[0].event !== "command.started") throw protocol("missing_started");
  const terminal = records[1];
  if (terminal.event === "command.result") {
    if (!exactKeys(terminal, ["version", "sequence", "event", "command", "invocation_id", "status", "exit_code", "data"])) {
      throw protocol("invalid_result");
    }
    validateResult(command, terminal, exitCode);
  } else if (terminal.event === "command.error") {
    if (!exactKeys(terminal, ["version", "sequence", "event", "command", "invocation_id", "status", "exit_code", "error"])) {
      throw protocol("invalid_error");
    }
    validateError(terminal, exitCode);
  }
  else throw protocol("missing_terminal");
  return Object.freeze({ records, terminal });
}

function validateResult(command, record, exitCode) {
  if (![ ["success", 0], ["content_diagnostics", 1] ].some(([status, code]) =>
    record.status === status && record.exit_code === code) ||
      record.exit_code !== exitCode || !record.data || typeof record.data !== "object" ||
      Array.isArray(record.data)) throw protocol("invalid_result");
  const diagnostics = record.data.diagnostics;
  if (command === "run" || command === "trace") {
    if (record.status !== "success" || !exactKeys(record.data, ["trace"]) || !validTrace(record.data.trace)) {
      throw protocol("invalid_runtime_result");
    }
    return;
  }
  if (!Array.isArray(diagnostics) || !diagnostics.every(validDiagnosticRecord)) throw protocol("invalid_diagnostics");
  if (record.status === "content_diagnostics") {
    if (!exactKeys(record.data, ["diagnostics"])) {
      throw protocol("invalid_content_result");
    }
    return;
  }
  if (command === "validate" && !exactKeys(record.data, ["diagnostics"]) ||
      command === "compile" && !exactKeys(record.data, ["diagnostics", "artifact"]) ||
      command === "extract" && !exactKeys(record.data, ["diagnostics", "artifact"]) &&
        !exactKeys(record.data, ["diagnostics", "entries"])) {
    throw protocol("invalid_result_shape");
  }
  if (command === "compile" && !validArtifact(record.data.artifact)) throw protocol("missing_artifact");
  if (command === "extract" && !validExtractData(record.data)) throw protocol("invalid_extract_result");
}

function validateError(record, exitCode) {
  if (record.status !== "failure" || record.exit_code !== 1 || exitCode !== 1 ||
      !validStructuredError(record.error)) throw protocol("invalid_error");
}

function validExtractData(data) {
  const artifact = data.artifact !== undefined;
  const entries = data.entries !== undefined;
  return artifact !== entries && (!artifact || validArtifact(data.artifact)) &&
    (!entries || Array.isArray(data.entries) && data.entries.every(validCatalogEntry));
}

function validArtifact(value) {
  return Boolean(value && typeof value === "object" && exactKeys(value, ["path", "size_bytes"]) && machinePath(value.path) &&
    integerInRange(value.size_bytes, "0", "18446744073709551615"));
}

function validCatalogEntry(value) {
  return Boolean(value && typeof value === "object" && exactKeys(value, ["context", "source_text", "plural_source_text", "comments", "reference"]) && string(value.context) &&
    string(value.source_text) && (value.plural_source_text === null ||
      typeof value.plural_source_text === "string") &&
    Array.isArray(value.comments) && value.comments.every((comment) => typeof comment === "string") &&
    (value.reference === null || validCatalogReference(value.reference)));
}

function validCatalogReference(value) {
  return Boolean(value && typeof value === "object" && exactKeys(value, ["file", "line", "column"]) && string(value.file) &&
    Number.isSafeInteger(value.line) && value.line > 0 &&
    Number.isSafeInteger(value.column) && value.column > 0);
}

function exactKeys(value, expected) {
  const keys = Object.keys(value).sort();
  return JSON.stringify(keys) === JSON.stringify(expected.slice().sort());
}

function machinePath(value) {
  return Boolean(value && typeof value === "object" && exactKeys(value, ["encoding", "value"]) &&
    ((value.encoding === "utf8" && typeof value.value === "string") ||
      (value.encoding === "unix_bytes" && typeof value.value === "string" &&
        /^[0-9a-f]*$/u.test(value.value) && value.value.length % 2 === 0) ||
      (value.encoding === "windows_wtf16" && Array.isArray(value.value) &&
        value.value.every((unit) => Number.isSafeInteger(unit) && unit >= 0 && unit <= 0xffff))));
}

function string(value) {
  return typeof value === "string" && value.length > 0;
}
