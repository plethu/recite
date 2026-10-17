import assert from "node:assert/strict";
import { test } from "bun:test";
import { parseLosslessJson } from "../src/command-protocol.ts";
import { WatchProtocolValidator } from "../src/watch-protocol.ts";

test("watch lifecycle requires an initial build and waiting between attempts", () => {
  const validator = new WatchProtocolValidator("watch", "watch-id");
  validator.consume(record(0, "watch.started", { project_root: root() }));
  assert.throws(() => validator.consume(record(1, "watch.waiting")), /invalid_watch_waiting/);
  const valid = new WatchProtocolValidator("watch", "watch-id");
  valid.consume(record(0, "watch.started", { project_root: root() }));
  valid.consume(record(1, "watch.build.started", { generation: 0, trigger: "initial" }));
  valid.consume(record(2, "watch.build.completed", completedData(0)));
  assert.throws(
    () =>
      valid.consume(
        record(3, "watch.build.started", {
          generation: 1,
          trigger: "input_changed",
        }),
      ),
    /invalid_build_started/,
  );
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
  noId.consume(
    recordFor(undefined, 1, "watch.stopped", {
      reason: { type: "fatal" },
      error: { category: "input", code: "missing_path", operation: "watch" },
    }),
  );
  noId.finish(1);

  const missing = new WatchProtocolValidator("watch", "expected-id");
  assert.throws(
    () => missing.consume(recordFor(undefined, 0, "watch.started", { project_root: root() })),
    /invocation_mismatch/,
  );
  const mismatched = new WatchProtocolValidator("watch", "expected-id");
  assert.throws(
    () => mismatched.consume(recordFor("other-id", 0, "watch.started", { project_root: root() })),
    /invocation_mismatch/,
  );
  const unexpected = new WatchProtocolValidator("watch");
  assert.throws(
    () => unexpected.consume(recordFor("other-id", 0, "watch.started", { project_root: root() })),
    /unexpected_invocation_id/,
  );
});

test("watch accepts startup fatal and rejects hostile typed values", () => {
  const fatal = new WatchProtocolValidator("watch", "fatal-id");
  fatal.consume(recordFor("fatal-id", 0, "watch.started", { project_root: root() }));
  fatal.consume(
    recordFor("fatal-id", 1, "watch.stopped", {
      reason: { type: "fatal" },
      error: { category: "input", code: "missing_path", operation: "watch" },
    }),
  );
  fatal.finish(1);

  const hostile = new WatchProtocolValidator("watch", "hostile-id");
  hostile.consume(recordFor("hostile-id", 0, "watch.started", { project_root: root() }));
  hostile.consume(
    recordFor("hostile-id", 1, "watch.build.started", { generation: 0, trigger: "initial" }),
  );
  const data = completedData(0);
  data.status = "made-up";
  assert.throws(
    () => hostile.consume(recordFor("hostile-id", 2, "watch.build.completed", data)),
    /invalid_build_completed/,
  );
});

test("watch accepts a producer-valid preparation failure in a fatal stop", () => {
  const validator = new WatchProtocolValidator("watch", "preparation-id");
  validator.consume(recordFor("preparation-id", 0, "watch.started", { project_root: root() }));
  validator.consume(
    recordFor("preparation-id", 1, "watch.stopped", {
      reason: { type: "fatal" },
      error: {
        category: "schema",
        code: "watch_preparation",
        operation: "load_schema",
        details: { type: "watch", kind: "schema_without_model" },
      },
    }),
  );
  validator.finish(1);
});

test("watch accepts a fatal stop after a completed build without waiting", () => {
  const fatal = new WatchProtocolValidator("watch", "completed-fatal-id");
  fatal.consume(recordFor("completed-fatal-id", 0, "watch.started", { project_root: root() }));
  fatal.consume(
    recordFor("completed-fatal-id", 1, "watch.build.started", {
      generation: 0,
      trigger: "initial",
    }),
  );
  fatal.consume(recordFor("completed-fatal-id", 2, "watch.build.completed", completedData(0)));
  fatal.consume(
    recordFor("completed-fatal-id", 3, "watch.stopped", {
      reason: { type: "fatal" },
      error: { category: "io", code: "io", operation: "watch" },
    }),
  );
  fatal.finish(1);

  const cancelled = new WatchProtocolValidator("watch", "completed-cancel-id");
  cancelled.consume(recordFor("completed-cancel-id", 0, "watch.started", { project_root: root() }));
  cancelled.consume(
    recordFor("completed-cancel-id", 1, "watch.build.started", {
      generation: 0,
      trigger: "initial",
    }),
  );
  cancelled.consume(recordFor("completed-cancel-id", 2, "watch.build.completed", completedData(0)));
  assert.throws(
    () =>
      cancelled.consume(
        recordFor("completed-cancel-id", 3, "watch.stopped", {
          reason: { type: "cancelled" },
        }),
      ),
    /invalid_watch_stopped/,
  );

  assert.throws(() => {
    const mismatch = new WatchProtocolValidator("watch", "completed-mismatch-id");
    mismatch.consume(
      recordFor("completed-mismatch-id", 0, "watch.started", { project_root: root() }),
    );
    mismatch.consume(
      recordFor("completed-mismatch-id", 1, "watch.build.started", {
        generation: 0,
        trigger: "initial",
      }),
    );
    mismatch.consume(
      recordFor("completed-mismatch-id", 2, "watch.build.completed", completedData(0)),
    );
    mismatch.consume(
      recordFor("completed-mismatch-id", 3, "watch.stopped", {
        reason: { type: "fatal" },
        error: { category: "io", code: "io", operation: "watch" },
      }),
    );
    mismatch.finish(0);
  }, /watch_exit_mismatch/);
});

test("watch accepts a fatal control-stream error emitted by the CLI", () => {
  const validator = new WatchProtocolValidator("watch", "control-fatal-id");
  validator.consume(recordFor("control-fatal-id", 0, "watch.started", { project_root: root() }));
  validator.consume(
    recordFor("control-fatal-id", 1, "watch.stopped", {
      reason: { type: "fatal" },
      error: { category: "io", code: "io", operation: "watch" },
    }),
  );
  validator.finish(1);
});

test("watch accepts typed stale freshness and operational build failure records", () => {
  const validator = new WatchProtocolValidator("watch", "stale-id");
  validator.consume(recordFor("stale-id", 0, "watch.started", { project_root: root() }));
  validator.consume(
    recordFor("stale-id", 1, "watch.build.started", { generation: 0, trigger: "initial" }),
  );
  const data = completedData(0);
  data.status = "stale";
  data.outcome = { type: "stale" };
  data.freshness = { type: "stale", reasons: [{ type: "build_generation" }] };
  data.publication = { type: "not_attempted", reason: "stale" };
  validator.consume(recordFor("stale-id", 2, "watch.build.completed", data));
  const failed = new WatchProtocolValidator("watch", "failed-id");
  failed.consume(recordFor("failed-id", 0, "watch.started", { project_root: root() }));
  failed.consume(
    recordFor("failed-id", 1, "watch.build.started", { generation: 0, trigger: "initial" }),
  );
  const failureData = completedData(0);
  failureData.status = "failed";
  failureData.outcome = { type: "operational_failure" };
  failureData.error = { category: "watch", code: "watch", operation: "build" };
  failed.consume(recordFor("failed-id", 2, "watch.build.completed", failureData));
});

test("watch artifact sizes accept an exact wide u64 value from the wire", () => {
  const validator = new WatchProtocolValidator("watch", "wide-watch");
  validator.consume(
    parseLosslessJson(
      JSON.stringify(recordFor("wide-watch", 0, "watch.started", { project_root: root() })),
    ),
  );
  validator.consume(
    parseLosslessJson(
      '{"version":1,"sequence":1,"event":"watch.build.started","command":"watch","invocation_id":"wide-watch",' +
        '"data":{"generation":0,"trigger":"initial"}}',
    ),
  );
  const data = completedData(0);
  data.artifacts = [{ path: root(), size_bytes: 0 }];
  const completed = JSON.stringify(
    recordFor("wide-watch", 2, "watch.build.completed", data),
  ).replace('"size_bytes":0', '"size_bytes":9007199254740993');
  validator.consume(parseLosslessJson(completed));
});

test("watch tagged DTO variants reject impossible extra fields", () => {
  const cancellation = new WatchProtocolValidator("watch", "cancel-extra");
  cancellation.consume(recordFor("cancel-extra", 0, "watch.started", { project_root: root() }));
  assert.throws(
    () =>
      cancellation.consume(
        recordFor("cancel-extra", 1, "watch.cancel.requested", {
          cancellation: { type: "user", by_generation: 9 },
        }),
      ),
    /invalid_cancel/,
  );

  const outcome = new WatchProtocolValidator("watch", "outcome-extra");
  outcome.consume(recordFor("outcome-extra", 0, "watch.started", { project_root: root() }));
  outcome.consume(
    recordFor("outcome-extra", 1, "watch.build.started", { generation: 0, trigger: "initial" }),
  );
  const data = completedData(0);
  data.outcome = { type: "fresh", injected: true };
  assert.throws(
    () => outcome.consume(recordFor("outcome-extra", 2, "watch.build.completed", data)),
    /invalid_build_completed/,
  );
});

test("watch rejects open-ended nested shapes and non-user cancellation", () => {
  const extraRoot = new WatchProtocolValidator("watch", "extra-root");
  assert.throws(
    () =>
      extraRoot.consume(
        recordFor("extra-root", 0, "watch.started", {
          project_root: { encoding: "utf8", value: "/project", extra: true },
        }),
      ),
    /invalid_watch_started/,
  );

  const control = new WatchProtocolValidator("watch", "control-id");
  control.consume(recordFor("control-id", 0, "watch.started", { project_root: root() }));
  assert.throws(
    () =>
      control.consume(
        recordFor("control-id", 1, "watch.control.error", {
          error: { type: "malformed", detail: "unexpected" },
        }),
      ),
    /invalid_control_error/,
  );
  assert.throws(
    () =>
      control.consume(
        recordFor("control-id", 2, "watch.cancel.requested", {
          cancellation: { type: "unknown" },
        }),
      ),
    /invalid_cancel/,
  );

  const notify = new WatchProtocolValidator("watch", "notify-id");
  notify.consume(recordFor("notify-id", 0, "watch.started", { project_root: root() }));
  assert.throws(
    () =>
      notify.consume(
        recordFor("notify-id", 1, "watch.notify.error", {
          error: { type: "watcher", detail: "unexpected" },
        }),
      ),
    /invalid_notify_error/,
  );
});

function root() {
  return { encoding: "utf8", value: "/project" };
}

function completedData(generation) {
  return {
    generation,
    snapshot_generation: 0,
    status: "succeeded",
    outcome: { type: "fresh" },
    inputs: [],
    diagnostics: [],
    artifacts: [],
    freshness: { type: "fresh" },
    publication: { type: "not_attempted", reason: "no_candidates" },
    recovery: [],
    restart_guidance: { type: "host_policy_required", decision: "unspecified" },
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
