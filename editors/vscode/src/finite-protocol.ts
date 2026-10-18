import {
  exactEnvelopeKeys,
  NdjsonRecordParser,
  protocol,
  validateEnvelope,
  type JsonRecord,
} from "./command-protocol.ts";
import {
  parseWire,
  startedSchema,
  resultSchema,
  errorSchema,
  diagnosticsDataSchema,
  artifactDataSchema,
  extractDataSchema,
  runtimeDataSchema,
  type FiniteData,
  type FiniteTerminal,
} from "./schemas/index.js";

const MAX_FINITE_BYTES = 32 * 1024 * 1024;
export interface FiniteResult {
  readonly records: JsonRecord[];
  readonly terminal: FiniteTerminal;
}

export function parseFiniteRecords(
  stdout: string | Uint8Array,
  command: string,
  invocationId: string | undefined,
  exitCode: number | null,
): FiniteResult {
  if (!["validate", "compile", "extract", "run", "trace"].includes(command))
    throw protocol("unsupported_command");
  const parser = new NdjsonRecordParser({ maxBytes: MAX_FINITE_BYTES });
  const records = parser.push(stdout);
  parser.finish();
  const [started, last] = records;
  if (records.length !== 2 || !started || !last) throw protocol("finite_record_count");
  validateEnvelope(started, command, invocationId, 0);
  validateEnvelope(last, command, invocationId, 1);
  parseWire(startedSchema, started, "missing_started");
  if (
    !exactEnvelopeKeys(
      started,
      ["version", "sequence", "event", "command", "invocation_id"],
      invocationId,
    )
  )
    throw protocol("missing_started");
  let terminal: FiniteTerminal;
  if (last.event === "command.error") {
    terminal = parseWire(errorSchema, last, "invalid_error");
    if (exitCode !== 1) throw protocol("invalid_error");
  } else if (last.event === "command.result") {
    const result = parseWire(resultSchema, last, "invalid_result");
    if (result.exit_code !== exitCode) throw protocol("invalid_result");
    let data: FiniteData;
    if (command === "run" || command === "trace") {
      if (result.status !== "success") throw protocol("invalid_runtime_result");
      data = parseWire(runtimeDataSchema, result.data, "invalid_runtime_result");
    } else if (result.status === "content_diagnostics" || command === "validate") {
      data = parseWire(diagnosticsDataSchema, result.data, "invalid_content_result");
    } else if (command === "compile") {
      data = parseWire(artifactDataSchema, result.data, "invalid_result_shape");
    } else {
      data = parseWire(extractDataSchema, result.data, "invalid_extract_result");
    }
    terminal = { ...result, data };
  } else throw protocol("missing_terminal");
  if (
    !exactEnvelopeKeys(
      last,
      last.event === "command.error"
        ? [
            "version",
            "sequence",
            "event",
            "command",
            "invocation_id",
            "status",
            "exit_code",
            "error",
          ]
        : [
            "version",
            "sequence",
            "event",
            "command",
            "invocation_id",
            "status",
            "exit_code",
            "data",
          ],
      invocationId,
    )
  )
    throw protocol("invalid_result");
  return Object.freeze({ records, terminal });
}
