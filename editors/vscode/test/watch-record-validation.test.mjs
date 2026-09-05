import test from "node:test";
import assert from "node:assert/strict";
import { validStructuredError } from "../src/watch-record-validation.js";

// Keep this table independently explicit: it is the reviewable v1 producer
// vocabulary, not a re-export of the validator's private allowlist.
const producerOperations = [
  "validate", "compile", "extract", "run", "trace",
  "load_asset", "load_catalog", "load_fixture", "inspect_asset", "resolve_path",
  "collect_inputs", "write_output", "read", "read_directory", "acknowledge_effect",
  "select_fixture_choice", "write",
  "watch", "discover_project", "start_watcher", "watch_project", "build",
  "read_project_input", "resolve_schema", "load_schema", "prepare_inputs",
  "validate_project", "prepare_request", "prepare_targets", "prepare_publisher",
  "resolve_project_root", "validate_target"
];

test("structured errors accept every producer operation and reject internal paths", () => {
  for (const operation of producerOperations) {
    assert.equal(validStructuredError({
      category: "input", code: "io", operation
    }), true, `producer operation should be accepted: ${operation}`);
  }
  for (const operation of ["control", "dispatch", "render", "not-a-wire-operation"]) {
    assert.equal(validStructuredError({
      category: "input", code: "io", operation
    }), false, `non-wire operation should be rejected: ${operation}`);
  }
});
