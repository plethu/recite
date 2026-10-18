import assert from "node:assert/strict";
import { test } from "bun:test";
import { LosslessInteger, NdjsonRecordParser, parseLosslessJson } from "../src/command-protocol.ts";
import { parseFiniteRecords } from "../src/finite-protocol.ts";

test("finite protocol validates exact records, terminal exit, and UTF-8 boundaries", () => {
  const started = JSON.stringify({
    version: 1,
    sequence: 0,
    event: "command.started",
    command: "validate",
    invocation_id: "id",
  });
  const result = JSON.stringify({
    version: 1,
    sequence: 1,
    event: "command.result",
    command: "validate",
    invocation_id: "id",
    status: "success",
    exit_code: 0,
    data: { diagnostics: [] },
  });
  assert.equal(
    parseFiniteRecords(`${started}\n${result}\n`, "validate", "id", 0).terminal.status,
    "success",
  );
  assert.throws(
    () => parseFiniteRecords(`${started}\n${result}\n${result}\n`, "validate", "id", 0),
    /finite_record_count/,
  );
  assert.throws(
    () => parseFiniteRecords(`${started}\nnot-json\n`, "validate", "id", 0),
    /invalid_json/,
  );
  assert.throws(
    () => parseFiniteRecords(`${started}\n${result}\n`, "validate", "id", 1),
    /invalid_result/,
  );
  const parser = new NdjsonRecordParser();
  const encoded = Buffer.from(`${started}\n`, "utf8");
  parser.push(encoded.subarray(0, encoded.length - 2));
  assert.equal(parser.push(encoded.subarray(encoded.length - 2))[0].event, "command.started");
  assert.throws(() => new NdjsonRecordParser().push(Buffer.from([0xff, 0x0a])), /invalid_utf8/);
});

test("finite protocol omits invocation metadata only when no ID is expected", () => {
  const started = { version: 1, sequence: 0, event: "command.started", command: "validate" };
  const result = {
    version: 1,
    sequence: 1,
    event: "command.result",
    command: "validate",
    status: "success",
    exit_code: 0,
    data: { diagnostics: [] },
  };
  const wire = `${JSON.stringify(started)}\n${JSON.stringify(result)}\n`;
  assert.equal(parseFiniteRecords(wire, "validate", undefined, 0).terminal.status, "success");
  assert.throws(
    () => parseFiniteRecords(wire, "validate", "expected-id", 0),
    /invocation_mismatch/,
  );

  const mismatched = `${JSON.stringify({ ...started, invocation_id: "other-id" })}\n${JSON.stringify(
    { ...result, invocation_id: "other-id" },
  )}\n`;
  assert.throws(
    () => parseFiniteRecords(mismatched, "validate", "expected-id", 0),
    /invocation_mismatch/,
  );
  assert.throws(
    () => parseFiniteRecords(mismatched, "validate", undefined, 0),
    /unexpected_invocation_id/,
  );
});

test("lossless JSON numbers preserve wide integers and cannot collide with source strings", () => {
  const value = parseLosslessJson(
    '{"min":-9223372036854775808,"max":9223372036854775807,"text":"\\u0000recite-lossless-integer:7"}',
  );
  assert(value.min instanceof LosslessInteger);
  assert(value.max instanceof LosslessInteger);
  assert.equal(value.min.raw, "-9223372036854775808");
  assert.equal(value.max.raw, "9223372036854775807");
  assert.equal(value.text, "\u0000recite-lossless-integer:7");
  assert.match(JSON.stringify(value), /9223372036854775807/);
  assert.equal(parseLosslessJson('{"value":1e3}').value, 1000);
  assert.throws(() => parseLosslessJson('{"value":1e}'), SyntaxError);
  assert.throws(() => parseLosslessJson('{"value":01}'), SyntaxError);
  assert.throws(() => parseLosslessJson('{"value":-01}'), SyntaxError);
});

test("finite runtime traces reject malformed nested events and metrics", () => {
  const started = {
    version: 1,
    sequence: 0,
    event: "command.started",
    command: "trace",
    invocation_id: "trace-id",
  };
  const trace = {
    asset_id: "asset",
    block: "start",
    events: [null],
    final_deferred_effects: [],
    metrics: {
      event_count: 0,
      line_count: 0,
      prompt_count: 0,
      choice_count: 0,
      condition_evaluation_count: 0,
      effect_count: { deferred: 0, immediate: 0, blocking: 0 },
      localization_lookup_count: 0,
      elapsed_traversal_time_ns: 0,
      max_serialized_session_size_bytes: 0,
    },
  };
  const result = {
    version: 1,
    sequence: 1,
    event: "command.result",
    command: "trace",
    invocation_id: "trace-id",
    status: "success",
    exit_code: 0,
    data: { trace },
  };
  assert.throws(
    () =>
      parseFiniteRecords(
        `${JSON.stringify(started)}\n${JSON.stringify(result)}\n`,
        "trace",
        "trace-id",
        0,
      ),
    /invalid_runtime_result/,
  );
  trace.events = [];
  trace.metrics.elapsed_traversal_time_ns = "bad";
  assert.throws(
    () =>
      parseFiniteRecords(
        `${JSON.stringify(started)}\n${JSON.stringify(result)}\n`,
        "trace",
        "trace-id",
        0,
      ),
    /invalid_runtime_result/,
  );
});

test("finite runtime traces accept the Rust nullable and omitted fields", () => {
  const started = {
    version: 1,
    sequence: 0,
    event: "command.started",
    command: "run",
    invocation_id: "trace-id",
  };
  const trace = {
    asset_id: "asset",
    block: "start",
    events: [
      {
        type: "line",
        line: {
          id: "line",
          source_text: "source",
          text: "text",
          speaker: null,
          metadata: [],
        },
      },
      {
        type: "effect",
        effect: {
          id: "effect",
          mode: "deferred",
          function: "save",
          args: [
            { type: "integer", value: 1 },
            { type: "integer", value: 2 },
          ],
          source_span: {
            file: "dialogue.recite",
            start_line: 1,
            start_column: 1,
            end_line: null,
            end_column: null,
          },
        },
      },
    ],
    final_deferred_effects: [],
  };
  const result = {
    version: 1,
    sequence: 1,
    event: "command.result",
    command: "run",
    invocation_id: "trace-id",
    status: "success",
    exit_code: 0,
    data: { trace },
  };
  assert.equal(
    parseFiniteRecords(
      `${JSON.stringify(started)}\n${JSON.stringify(result)}\n`,
      "run",
      "trace-id",
      0,
    ).terminal.status,
    "success",
  );
  const hostile = structuredClone(trace);
  hostile.events[1].effect.args[0] = { type: "boolean", value: "true" };
  assert.throws(
    () =>
      parseFiniteRecords(
        `${JSON.stringify(started)}\n${JSON.stringify({ ...result, data: { trace: hostile } })}\n`,
        "run",
        "trace-id",
        0,
      ),
    /invalid_runtime_result/,
  );
});

test("finite runtime traces preserve i64 and u128 wire values", () => {
  const started = {
    version: 1,
    sequence: 0,
    event: "command.started",
    command: "trace",
    invocation_id: "wide-id",
  };
  const trace = {
    asset_id: "asset",
    block: "start",
    events: [
      {
        type: "effect",
        effect: {
          id: "effect",
          mode: "deferred",
          function: "save",
          args: [
            { type: "integer", value: 1 },
            { type: "integer", value: 2 },
          ],
          source_span: {
            file: "dialogue.recite",
            start_line: 1,
            start_column: 1,
            end_line: null,
            end_column: null,
          },
        },
      },
    ],
    final_deferred_effects: [],
    metrics: {
      event_count: 1,
      line_count: 0,
      prompt_count: 0,
      choice_count: 0,
      condition_evaluation_count: 0,
      effect_count: { deferred: 1, immediate: 0, blocking: 0 },
      localization_lookup_count: 0,
      elapsed_traversal_time_ns: 0,
      max_serialized_session_size_bytes: 0,
    },
  };
  const result = {
    version: 1,
    sequence: 1,
    event: "command.result",
    command: "trace",
    invocation_id: "wide-id",
    status: "success",
    exit_code: 0,
    data: { trace },
  };
  const wire = JSON.stringify(result)
    .replace('"value":1', '"value":-9223372036854775808')
    .replace('"value":2', '"value":9223372036854775807')
    .replace(
      '"elapsed_traversal_time_ns":0',
      '"elapsed_traversal_time_ns":340282366920938463463374607431768211455',
    );
  const parsed = parseFiniteRecords(`${JSON.stringify(started)}\n${wire}\n`, "trace", "wide-id", 0);
  assert.equal(
    parsed.terminal.data.trace.events[0].effect.args[0].value.raw,
    "-9223372036854775808",
  );
  assert.equal(
    parsed.terminal.data.trace.events[0].effect.args[1].value.raw,
    "9223372036854775807",
  );
  assert.equal(
    parsed.terminal.data.trace.metrics.elapsed_traversal_time_ns.raw,
    "340282366920938463463374607431768211455",
  );
  assert.doesNotThrow(() => JSON.stringify(parsed.terminal.data));
});
