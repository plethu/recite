import test from "node:test";
import assert from "node:assert/strict";
import { LosslessInteger, NdjsonRecordParser, parseLosslessJson } from "../src/command-protocol.js";
import { parseFiniteRecords } from "../src/finite-protocol.js";
import { WatchProtocolValidator } from "../src/watch-protocol.js";

test("finite protocol validates exact records, terminal exit, and UTF-8 boundaries", () => {
  const started = JSON.stringify({
    version: 1, sequence: 0, event: "command.started", command: "validate", invocation_id: "id"
  });
  const result = JSON.stringify({
    version: 1, sequence: 1, event: "command.result", command: "validate", invocation_id: "id",
    status: "success", exit_code: 0, data: { diagnostics: [] }
  });
  assert.equal(parseFiniteRecords(`${started}\n${result}\n`, "validate", "id", 0).terminal.status, "success");
  assert.throws(() => parseFiniteRecords(`${started}\n${result}\n${result}\n`, "validate", "id", 0), /finite_record_count/);
  assert.throws(() => parseFiniteRecords(`${started}\nnot-json\n`, "validate", "id", 0), /invalid_json/);
  assert.throws(() => parseFiniteRecords(`${started}\n${result}\n`, "validate", "id", 1), /invalid_result/);
  const parser = new NdjsonRecordParser();
  const encoded = Buffer.from(`${started}\n`, "utf8");
  parser.push(encoded.subarray(0, encoded.length - 2));
  assert.equal(parser.push(encoded.subarray(encoded.length - 2))[0].event, "command.started");
  assert.throws(() => new NdjsonRecordParser().push(Buffer.from([0xff, 0x0a])), /invalid_utf8/);
});

test("finite protocol omits invocation metadata only when no ID is expected", () => {
  const started = { version: 1, sequence: 0, event: "command.started", command: "validate" };
  const result = {
    version: 1, sequence: 1, event: "command.result", command: "validate",
    status: "success", exit_code: 0, data: { diagnostics: [] }
  };
  const wire = `${JSON.stringify(started)}\n${JSON.stringify(result)}\n`;
  assert.equal(parseFiniteRecords(wire, "validate", undefined, 0).terminal.status, "success");
  assert.throws(() => parseFiniteRecords(wire, "validate", "expected-id", 0), /invocation_mismatch/);

  const mismatched = `${JSON.stringify({ ...started, invocation_id: "other-id" })}\n${JSON.stringify({ ...result, invocation_id: "other-id" })}\n`;
  assert.throws(() => parseFiniteRecords(mismatched, "validate", "expected-id", 0), /invocation_mismatch/);
  assert.throws(() => parseFiniteRecords(mismatched, "validate", undefined, 0), /unexpected_invocation_id/);
});

test("lossless JSON numbers preserve wide integers and cannot collide with source strings", () => {
  const value = parseLosslessJson(
    '{"min":-9223372036854775808,"max":9223372036854775807,"text":"\\u0000recite-lossless-integer:7"}'
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
  const started = { version: 1, sequence: 0, event: "command.started", command: "trace", invocation_id: "trace-id" };
  const trace = {
    asset_id: "asset", block: "start", events: [null], final_deferred_effects: [],
    metrics: { event_count: 0, line_count: 0, prompt_count: 0, choice_count: 0,
      condition_evaluation_count: 0, effect_count: { deferred: 0, immediate: 0, blocking: 0 },
      localization_lookup_count: 0, elapsed_traversal_time_ns: 0,
      max_serialized_session_size_bytes: 0 }
  };
  const result = { version: 1, sequence: 1, event: "command.result", command: "trace", invocation_id: "trace-id",
    status: "success", exit_code: 0, data: { trace } };
  assert.throws(() => parseFiniteRecords(`${JSON.stringify(started)}\n${JSON.stringify(result)}\n`, "trace", "trace-id", 0),
    /invalid_runtime_result/);
  trace.events = [];
  trace.metrics.elapsed_traversal_time_ns = "bad";
  assert.throws(() => parseFiniteRecords(`${JSON.stringify(started)}\n${JSON.stringify(result)}\n`, "trace", "trace-id", 0),
    /invalid_runtime_result/);
});

test("finite runtime traces accept the Rust nullable and omitted fields", () => {
  const started = { version: 1, sequence: 0, event: "command.started", command: "run", invocation_id: "trace-id" };
  const trace = {
    asset_id: "asset", block: "start",
    events: [{ type: "line", line: {
      id: "line", source_text: "source", text: "text", speaker: null, metadata: []
    } }, { type: "effect", effect: {
      id: "effect", mode: "deferred", function: "save", args: [
        { type: "integer", value: 1 }, { type: "integer", value: 2 }
      ],
      source_span: { file: "dialogue.recite", start_line: 1, start_column: 1, end_line: null, end_column: null }
    } }],
    final_deferred_effects: []
  };
  const result = { version: 1, sequence: 1, event: "command.result", command: "run", invocation_id: "trace-id",
    status: "success", exit_code: 0, data: { trace } };
  assert.equal(parseFiniteRecords(`${JSON.stringify(started)}\n${JSON.stringify(result)}\n`, "run", "trace-id", 0)
    .terminal.status, "success");
  const hostile = structuredClone(trace);
  hostile.events[1].effect.args[0] = { type: "boolean", value: "true" };
  assert.throws(() => parseFiniteRecords(`${JSON.stringify(started)}\n${JSON.stringify({ ...result, data: { trace: hostile } })}\n`,
    "run", "trace-id", 0), /invalid_runtime_result/);
});

test("finite runtime traces preserve i64 and u128 wire values", () => {
  const started = { version: 1, sequence: 0, event: "command.started", command: "trace", invocation_id: "wide-id" };
  const trace = {
    asset_id: "asset", block: "start", events: [{ type: "effect", effect: {
      id: "effect", mode: "deferred", function: "save", args: [
        { type: "integer", value: 1 }, { type: "integer", value: 2 }
      ],
      source_span: { file: "dialogue.recite", start_line: 1, start_column: 1, end_line: null, end_column: null }
    } }], final_deferred_effects: [], metrics: {
      event_count: 1, line_count: 0, prompt_count: 0, choice_count: 0,
      condition_evaluation_count: 0, effect_count: { deferred: 1, immediate: 0, blocking: 0 },
      localization_lookup_count: 0, elapsed_traversal_time_ns: 0, max_serialized_session_size_bytes: 0
    }
  };
  const result = { version: 1, sequence: 1, event: "command.result", command: "trace", invocation_id: "wide-id",
    status: "success", exit_code: 0, data: { trace } };
  const wire = JSON.stringify(result)
    .replace('"value":1', '"value":-9223372036854775808')
    .replace('"value":2', '"value":9223372036854775807')
    .replace('"elapsed_traversal_time_ns":0', '"elapsed_traversal_time_ns":340282366920938463463374607431768211455');
  const parsed = parseFiniteRecords(`${JSON.stringify(started)}\n${wire}\n`, "trace", "wide-id", 0);
  assert.equal(parsed.terminal.data.trace.events[0].effect.args[0].value.raw, "-9223372036854775808");
  assert.equal(parsed.terminal.data.trace.events[0].effect.args[1].value.raw, "9223372036854775807");
  assert.equal(parsed.terminal.data.trace.metrics.elapsed_traversal_time_ns.raw,
    "340282366920938463463374607431768211455");
  assert.doesNotThrow(() => JSON.stringify(parsed.terminal.data));
});

test("watch lifecycle requires an initial build and waiting between attempts", () => {
  const validator = new WatchProtocolValidator("watch", "watch-id");
  validator.consume(record(0, "watch.started", { project_root: root() }));
  assert.throws(() => validator.consume(record(1, "watch.waiting")), /invalid_watch_waiting/);
  const valid = new WatchProtocolValidator("watch", "watch-id");
  valid.consume(record(0, "watch.started", { project_root: root() }));
  valid.consume(record(1, "watch.build.started", { generation: 0, trigger: "initial" }));
  valid.consume(record(2, "watch.build.completed", completedData(0)));
  assert.throws(() => valid.consume(record(3, "watch.build.started", {
    generation: 1, trigger: "input_changed"
  })), /invalid_build_started/);
  const complete = new WatchProtocolValidator("watch", "watch-id");
  complete.consume(record(0, "watch.started", { project_root: root() }));
  complete.consume(record(1, "watch.build.started", { generation: 0, trigger: "initial" }));
  complete.consume(record(2, "watch.build.completed", completedData(0)));
  complete.consume(record(3, "watch.waiting"));
  complete.consume(record(4, "watch.cancel.requested", { cancellation: { type: "user" } }));
  complete.consume(record(5, "watch.stopped", { reason: { type: "cancelled" } }));
  complete.finish(0);
});

test("watch protocol omits invocation metadata only when no ID is expected", () => {
  const noId = new WatchProtocolValidator("watch");
  noId.consume(recordFor(undefined, 0, "watch.started", { project_root: root() }));
  noId.consume(recordFor(undefined, 1, "watch.stopped", {
    reason: { type: "fatal" }, error: { category: "input", code: "missing_path", operation: "watch" }
  }));
  noId.finish(1);

  const missing = new WatchProtocolValidator("watch", "expected-id");
  assert.throws(() => missing.consume(recordFor(undefined, 0, "watch.started", { project_root: root() })), /invocation_mismatch/);
  const mismatched = new WatchProtocolValidator("watch", "expected-id");
  assert.throws(() => mismatched.consume(recordFor("other-id", 0, "watch.started", { project_root: root() })), /invocation_mismatch/);
  const unexpected = new WatchProtocolValidator("watch");
  assert.throws(() => unexpected.consume(recordFor("other-id", 0, "watch.started", { project_root: root() })), /unexpected_invocation_id/);
});

test("watch accepts startup fatal and rejects hostile typed values", () => {
  const fatal = new WatchProtocolValidator("watch", "fatal-id");
  fatal.consume(recordFor("fatal-id", 0, "watch.started", { project_root: root() }));
  fatal.consume(recordFor("fatal-id", 1, "watch.stopped", {
    reason: { type: "fatal" }, error: { category: "input", code: "missing_path", operation: "watch" }
  }));
  fatal.finish(1);

  const hostile = new WatchProtocolValidator("watch", "hostile-id");
  hostile.consume(recordFor("hostile-id", 0, "watch.started", { project_root: root() }));
  hostile.consume(recordFor("hostile-id", 1, "watch.build.started", { generation: 0, trigger: "initial" }));
  const data = completedData(0);
  data.status = "made-up";
  assert.throws(() => hostile.consume(recordFor("hostile-id", 2, "watch.build.completed", data)), /invalid_build_completed/);
});

test("watch accepts a producer-valid preparation failure in a fatal stop", () => {
  const validator = new WatchProtocolValidator("watch", "preparation-id");
  validator.consume(recordFor("preparation-id", 0, "watch.started", { project_root: root() }));
  validator.consume(recordFor("preparation-id", 1, "watch.stopped", {
    reason: { type: "fatal" },
    error: {
      category: "schema", code: "watch_preparation", operation: "load_schema",
      details: { type: "watch", kind: "schema_without_model" }
    }
  }));
  validator.finish(1);
});

test("watch accepts a fatal control-stream error emitted by the CLI", () => {
  const validator = new WatchProtocolValidator("watch", "control-fatal-id");
  validator.consume(recordFor("control-fatal-id", 0, "watch.started", { project_root: root() }));
  validator.consume(recordFor("control-fatal-id", 1, "watch.stopped", {
    reason: { type: "fatal" },
    error: { category: "io", code: "io", operation: "watch" }
  }));
  validator.finish(1);
});

test("watch accepts typed stale freshness and operational build failure records", () => {
  const validator = new WatchProtocolValidator("watch", "stale-id");
  validator.consume(recordFor("stale-id", 0, "watch.started", { project_root: root() }));
  validator.consume(recordFor("stale-id", 1, "watch.build.started", { generation: 0, trigger: "initial" }));
  const data = completedData(0);
  data.status = "stale";
  data.outcome = { type: "stale" };
  data.freshness = { type: "stale", reasons: [{ type: "build_generation" }] };
  data.publication = { type: "not_attempted", reason: "stale" };
  validator.consume(recordFor("stale-id", 2, "watch.build.completed", data));
  const failed = new WatchProtocolValidator("watch", "failed-id");
  failed.consume(recordFor("failed-id", 0, "watch.started", { project_root: root() }));
  failed.consume(recordFor("failed-id", 1, "watch.build.started", { generation: 0, trigger: "initial" }));
  const failureData = completedData(0);
  failureData.status = "failed";
  failureData.outcome = { type: "operational_failure" };
  failureData.error = { category: "watch", code: "watch", operation: "build" };
  failed.consume(recordFor("failed-id", 2, "watch.build.completed", failureData));
});

test("watch artifact sizes accept an exact wide u64 value from the wire", () => {
  const validator = new WatchProtocolValidator("watch", "wide-watch");
  validator.consume(parseLosslessJson(JSON.stringify(recordFor("wide-watch", 0, "watch.started", { project_root: root() }))));
  validator.consume(parseLosslessJson(
    '{"version":1,"sequence":1,"event":"watch.build.started","command":"watch","invocation_id":"wide-watch",' +
    '"data":{"generation":0,"trigger":"initial"}}'
  ));
  const data = completedData(0);
  data.artifacts = [{ path: root(), size_bytes: 0 }];
  const completed = JSON.stringify(recordFor("wide-watch", 2, "watch.build.completed", data))
    .replace('"size_bytes":0', '"size_bytes":9007199254740993');
  validator.consume(parseLosslessJson(completed));
});

test("watch tagged DTO variants reject impossible extra fields", () => {
  const cancellation = new WatchProtocolValidator("watch", "cancel-extra");
  cancellation.consume(recordFor("cancel-extra", 0, "watch.started", { project_root: root() }));
  assert.throws(() => cancellation.consume(recordFor("cancel-extra", 1, "watch.cancel.requested", {
    cancellation: { type: "user", by_generation: 9 }
  })), /invalid_cancel/);

  const outcome = new WatchProtocolValidator("watch", "outcome-extra");
  outcome.consume(recordFor("outcome-extra", 0, "watch.started", { project_root: root() }));
  outcome.consume(recordFor("outcome-extra", 1, "watch.build.started", { generation: 0, trigger: "initial" }));
  const data = completedData(0);
  data.outcome = { type: "fresh", injected: true };
  assert.throws(() => outcome.consume(recordFor("outcome-extra", 2, "watch.build.completed", data)), /invalid_build_completed/);
});

test("watch rejects open-ended nested shapes and non-user cancellation", () => {
  const extraRoot = new WatchProtocolValidator("watch", "extra-root");
  assert.throws(() => extraRoot.consume(recordFor("extra-root", 0, "watch.started", {
    project_root: { encoding: "utf8", value: "/project", extra: true }
  })), /invalid_watch_started/);

  const control = new WatchProtocolValidator("watch", "control-id");
  control.consume(recordFor("control-id", 0, "watch.started", { project_root: root() }));
  assert.throws(() => control.consume(recordFor("control-id", 1, "watch.control.error", {
    error: { type: "malformed", detail: "unexpected" }
  })), /invalid_control_error/);
  assert.throws(() => control.consume(recordFor("control-id", 2, "watch.cancel.requested", {
    cancellation: { type: "unknown" }
  })), /invalid_cancel/);

  const notify = new WatchProtocolValidator("watch", "notify-id");
  notify.consume(recordFor("notify-id", 0, "watch.started", { project_root: root() }));
  assert.throws(() => notify.consume(recordFor("notify-id", 1, "watch.notify.error", {
    error: { type: "watcher", detail: "unexpected" }
  })), /invalid_notify_error/);
});

function root() {
  return { encoding: "utf8", value: "/project" };
}

function completedData(generation) {
  return {
    generation, snapshot_generation: 0, status: "succeeded", outcome: { type: "fresh" },
    inputs: [], diagnostics: [], artifacts: [], freshness: { type: "fresh" },
    publication: { type: "not_attempted", reason: "no_candidates" }, recovery: [],
    restart_guidance: { type: "host_policy_required", decision: "unspecified" }
  };
}

function record(sequence, event, data = {}) {
  return recordFor("watch-id", sequence, event, data);
}

function recordFor(invocationId, sequence, event, data) {
  const record = { version: 1, sequence, event, command: "watch", data };
  if (invocationId !== undefined) record.invocation_id = invocationId;
  return record;
}
