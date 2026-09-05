local protocol = require("recite.command_protocol")
local finite = require("recite.finite_protocol")
local watch_protocol = require("recite.watch_protocol")
local command_diagnostics = require("recite.command_diagnostics")
local diagnostic_protocol = require("recite.diagnostic_protocol")

vim.notify = function() end
local function fail(message) error("Neovim command protocol check: " .. message, 0) end
local function assert_true(value, message) if not value then fail(message) end end
local function expect_error(function_, code)
  local ok, error = pcall(function_)
  assert_true(not ok and error.code == code, "expected protocol error " .. code)
end
local function expect_failure(function_, text)
  local ok, error = pcall(function_)
  assert_true(not ok and tostring(error):find(text, 1, true) ~= nil, "expected failure containing " .. text)
end

local parser = protocol.new_parser()
local records = parser:push('{"version":1,"sequence":18446744073709551615,"event":"x","command":"x","invocation_id":"i"}\n')
assert_true(#records == 1 and protocol.integer_raw(records[1].sequence) == "18446744073709551615", "u64 sequence lost precision")
parser:push('{"value":340282366920938463463374607431768211455}')
expect_error(function() parser:finish() end, "truncated_record")
expect_error(function() protocol.new_parser():push('{"value":01}\n') end, "invalid_json")
expect_error(function() protocol.new_parser(4):push("12345") end, "record_too_large")
local empty_object, empty_array = protocol.parse_json("{}"), protocol.parse_json("[]")
assert_true(protocol.object(empty_object) and not protocol.array(empty_object), "JSON empty object lost its object shape")
assert_true(protocol.array(empty_array) and not protocol.object(empty_array), "JSON empty array lost its array shape")
expect_failure(function() protocol.parse_json('{"x":1,"x":2}') end, "duplicate JSON object key: x")
expect_failure(function() protocol.parse_json('{"nested":{"x":1,"x":2}}') end, "duplicate JSON object key: x")
assert_true(protocol.is_integer(9007199254740991), "safe native integer was rejected")
assert_true(not protocol.is_integer(9007199254740992), "unsafe native integer was accepted")
local opaque = protocol.integer("18446744073709551615")
assert_true(protocol.integer_raw(opaque) == "18446744073709551615" and not protocol.is_integer({ raw = "18446744073709551615" }), "wide integer representation was forgeable")
local escaped_marker = protocol.parse_json('{"value":"__recite_lossless_integer:\\u0031\\u0032\\u0033"}')
assert_true(type(escaped_marker.value) == "string", "escaped integer marker string was restored as an integer")
local nested_marker = protocol.parse_json('{"value":{"nested":"__recite_lossless_integer:1"}}')
assert_true(type(nested_marker.value.nested) == "string", "nested marker string was restored as an integer")
local marker_key = protocol.parse_json('{"__recite_lossless_integer:1":18446744073709551615}')
assert_true(type(next(marker_key)) == "string", "integer marker key was restored as an integer")
local collision = protocol.parse_json('{"value":"__recite_lossless_integer__1","wide":18446744073709551615}')
assert_true(type(collision.value) == "string" and protocol.integer_raw(collision.wide) == "18446744073709551615", "deterministic placeholder collision was not excluded from the original inventory")

local wire_codes = {
  "core_value", "compile", "compiled_value", "decode_asset", "diagnostics", "diagnostic_rendering",
  "dialogue_catalog_conflict", "dialogue_catalog_plural_forms_conflict", "dialogue_catalog_malformed",
  "dialogue_catalog_missing_locale", "dialogue_catalog_spec_invalid", "dialogue_locale_invalid",
  "diagnostic_code_malformed", "diagnostic_code_unknown", "fixture_choice_index_out_of_range",
  "fixture_choice_not_in_prompt", "ambiguous_fixture_choice", "fixture_toml", "asset_metadata",
  "asset_not_file", "io", "malformed_compiled_asset", "missing_path", "invalid_project_root",
  "missing_fixture_choice", "no_inputs", "output_overwrites_input", "play_eof", "play_invalid_input",
  "play_interrupted", "play_tui_requires_terminal", "read", "read_directory", "runtime", "preview",
  "blocking_effect_needs_acknowledgement", "bench", "benchmark", "bench_json", "trace_json",
  "schema_inspection", "user_config", "project_discovery", "ui_catalog", "watch", "watch_coordinator",
  "watch_recovery", "write", "watch_preparation", "watch_publisher",
}
for _, code in ipairs(wire_codes) do
  assert_true(protocol.valid_error({ category = "input", code = code, operation = "validate" }), "authoritative error code was rejected: " .. code)
end
assert_true(not protocol.valid_error({ category = "input", code = "structured_stderr", operation = "validate" }), "transport stderr error leaked into the authoritative error-code set")
local wire_operations = { "validate", "compile", "extract", "run", "trace", "watch", "load_asset", "load_catalog", "load_fixture", "inspect_asset", "collect_inputs", "write_output", "acknowledge_effect", "resolve_path", "discover_project", "select_fixture_choice", "start_watcher", "watch_project", "control", "build", "read", "read_directory", "write", "load_schema", "read_project_input", "prepare_inputs", "validate_project", "prepare_request", "prepare_targets", "resolve_schema", "prepare_publisher", "resolve_project_root", "validate_target" }
for _, operation in ipairs(wire_operations) do
  assert_true(protocol.valid_error({ category = "input", code = "missing_path", operation = operation }), "authoritative operation was rejected: " .. operation)
end
assert_true(not protocol.valid_error({ category = "input", code = "missing_path", operation = "validate", path = vim.NIL }), "explicit null path was accepted")
assert_true(not protocol.valid_error({ category = "input", code = "missing_path", operation = "validate", related_path = vim.NIL }), "explicit null related_path was accepted")
assert_true(not protocol.valid_error({ category = "input", code = "missing_path", operation = "validate", details = vim.NIL }), "explicit null details was accepted")
local many_records = protocol.new_parser(8)
assert_true(#many_records:push("{}\n{}\n{}\n") == 3, "per-record limit was incorrectly cumulative")
expect_error(function() protocol.new_parser(protocol.MAX_RECORD_BYTES, protocol.MAX_FINITE_BYTES):push(string.rep("x", protocol.MAX_RECORD_BYTES + 1)) end, "record_too_large")
local bounded_finite = protocol.new_parser(8, 12)
assert_true(#bounded_finite:push("{}\n{}\n{}\n") == 3, "bounded finite payload rejected valid records")
expect_error(function() bounded_finite:push("{}\n{}\n{}\n") end, "stream_too_large")

local canonical = { id = "diagnostic-config-101", arguments = { detail = { type = "string", value = "missing" } } }
assert_true(diagnostic_protocol.valid_presentation(canonical), "canonical diagnostic presentation was rejected")
assert_true(diagnostic_protocol.render(canonical) == "project manifest not found: missing", "canonical diagnostic was not rendered")
assert_true(not diagnostic_protocol.valid_presentation({ id = "diagnostic-config-101", arguments = { wrong = { type = "string", value = "x" } } }), "unknown diagnostic argument was accepted")
assert_true(not diagnostic_protocol.valid_presentation({ id = "diagnostic-config-101", arguments = { detail = { type = "integer", value = 1 } } }), "diagnostic argument type mismatch was accepted")
local run_records = {
  { version = 1, sequence = 0, event = "command.started", command = "run", invocation_id = "run-1" },
  { version = 1, sequence = 1, event = "command.result", command = "run", invocation_id = "run-1", status = "success", exit_code = 0, data = { trace = { asset_id = "asset", block = "start", events = {}, final_deferred_effects = {} } } },
}
assert_true(finite.parse(run_records, "run", "run-1", 0).terminal.data.trace.block == "start", "run trace shape was not accepted")
run_records[2].sequence = 2
expect_error(function() finite.parse(run_records, "run", "run-1", 0) end, "invalid_envelope")
assert_true(diagnostic_protocol.valid_source_position({ line = 4294967295, column = 4294967295 }), "u32 source position boundary was rejected")
assert_true(not diagnostic_protocol.valid_source_position({ line = 4294967296, column = 1 }), "wide source position line was accepted")
local trace_protocol = require("recite.trace_protocol")
assert_true(trace_protocol.valid_source_span({ file = "x", start_line = 4294967295, start_column = 1, end_line = 4294967295, end_column = 4294967295 }), "trace u32 source span boundary was rejected")
assert_true(not trace_protocol.valid_source_span({ file = "x", start_line = 4294967296, start_column = 1, end_line = vim.NIL, end_column = vim.NIL }), "trace wide source span line was accepted")
local extract_started = { version = 1, sequence = 0, event = "command.started", command = "extract", invocation_id = "extract-width" }
local extract_terminal = { version = 1, sequence = 1, event = "command.result", command = "extract", invocation_id = "extract-width", status = "success", exit_code = 0, data = { diagnostics = {}, entries = { { context = "", source_text = "source", plural_source_text = vim.NIL, comments = {}, reference = { file = "x", line = 4294967295, column = 4294967295 } } } } }
assert_true(finite.parse({ extract_started, extract_terminal }, "extract", "extract-width", 0).terminal.data.entries[1].reference.line ~= nil, "extract u32 catalog reference boundary was rejected")
extract_terminal.data.entries[1].reference.line = protocol.integer("4294967296")
expect_error(function() finite.parse({ extract_started, extract_terminal }, "extract", "extract-width", 0) end, "invalid_result")

local source_path = vim.fn.tempname() .. ".recite"
local handle = assert(io.open(source_path, "wb")); handle:write("😀East\r\n"); handle:close()
local bufnr = vim.fn.bufadd(source_path); vim.fn.bufload(bufnr); vim.bo[bufnr].modified = false
local namespace, known = command_diagnostics.new_namespace(), {}
local diagnostic = { version = 1, code = "RECITE_X001", severity = "error", span = { file = source_path, start = { line = 1, column = 2 }, ["end"] = { line = 1, column = 2 } }, presentation = { id = "diagnostic-parse-001", arguments = vim.empty_dict() }, related = {}, help = vim.NIL, explanation = vim.NIL, compatibility_message = "east" }
command_diagnostics.replace(namespace, { diagnostic }, vim.fn.fnamemodify(source_path, ":h"), known)
local projected = vim.diagnostic.get(bufnr, { namespace = namespace })
assert_true(#projected == 1 and projected[1].col == 4 and projected[1].end_col == 5, "CRLF/non-BMP byte range projection was wrong")
vim.bo[bufnr].modified = true; command_diagnostics.replace(namespace, { diagnostic }, vim.fn.fnamemodify(source_path, ":h"), known)
assert_true(#vim.diagnostic.get(bufnr, { namespace = namespace }) == 0, "dirty overlay received disk CLI diagnostics")
vim.api.nvim_buf_delete(bufnr, { force = true }); os.remove(source_path)

local root = vim.env.RECITE_TEST_PROJECT
local inputs = require("recite.command_inputs")
assert_true(inputs.inside(root .. "/dialogue/../core_language_spike.recite", root), "normalized child path was rejected")
assert_true(not inputs.inside(root .. "/../outside.recite", root), "outside-root path escaped input fencing")
assert_true(inputs.paths_for({}, root)[1] == root, "omitted paths did not default to the project root")
assert_true(inputs.paths_for({ paths = {} }, root) == nil, "empty paths silently executed zero inputs")
local byte_root = root .. "/nonutf8-" .. string.char(0x80)
assert_true(vim.fn.mkdir(byte_root, "p") == 1, "non-UTF8 project root could not be created")
local function hex_bytes(value) local output = {}; for index = 1, #value do output[#output + 1] = string.format("%02x", value:byte(index)) end; return table.concat(output) end
local byte_validator = watch_protocol.new("watch-bytes", byte_root)
byte_validator:consume({ version = 1, sequence = 0, event = "watch.started", command = "watch", invocation_id = "watch-bytes", data = { project_root = { encoding = "unix_bytes", value = hex_bytes(byte_root) } } })
assert_true(byte_validator.project_root == byte_root, "unix_bytes project root did not round-trip")
local started = { version = 1, sequence = 0, event = "watch.started", command = "watch", invocation_id = "watch-1", data = { project_root = { encoding = "utf8", value = root } } }
local completed = { version = 1, sequence = 2, event = "watch.build.completed", command = "watch", invocation_id = "watch-1", data = { generation = 0, snapshot_generation = vim.NIL, status = "succeeded", outcome = { type = "fresh" }, inputs = {}, diagnostics = {}, artifacts = {}, freshness = { type = "fresh" }, publication = { type = "not_attempted", reason = "no_candidates" }, recovery = {}, restart_guidance = { type = "host_policy_required", decision = "unspecified" } } }
local validator = watch_protocol.new("watch-1", root)
validator:consume(started); validator:consume({ version = 1, sequence = 1, event = "watch.build.started", command = "watch", invocation_id = "watch-1", data = { generation = 0, trigger = "initial" } }); validator:consume(completed); validator:consume({ version = 1, sequence = 3, event = "watch.waiting", command = "watch", invocation_id = "watch-1", data = vim.empty_dict() }); validator:consume({ version = 1, sequence = 4, event = "watch.cancel.requested", command = "watch", invocation_id = "watch-1", data = { cancellation = { type = "user" } } }); validator:consume({ version = 1, sequence = 5, event = "watch.stopped", command = "watch", invocation_id = "watch-1", data = { reason = { type = "cancelled" } } }); validator:finish(0)
expect_error(function() validator:consume(started) end, "records_after_stopped")
local fatal_validator = watch_protocol.new("watch-fatal", root); local fatal_started = vim.deepcopy(started); fatal_started.invocation_id = "watch-fatal"; fatal_validator:consume(fatal_started); fatal_validator:consume({ version = 1, sequence = 1, event = "watch.build.started", command = "watch", invocation_id = "watch-fatal", data = { generation = 0, trigger = "initial" } }); local fatal_completed = vim.deepcopy(completed); fatal_completed.invocation_id = "watch-fatal"; fatal_validator:consume(fatal_completed); fatal_validator:consume({ version = 1, sequence = 3, event = "watch.stopped", command = "watch", invocation_id = "watch-fatal", data = { reason = { type = "fatal" }, error = { category = "input", code = "missing_path", operation = "resolve_path" } } }); fatal_validator:finish(1)
local fatal_wrong_exit = watch_protocol.new("watch-fatal-wrong", root); local fatal_wrong_started = vim.deepcopy(fatal_started); fatal_wrong_started.invocation_id = "watch-fatal-wrong"; fatal_wrong_exit:consume(fatal_wrong_started); fatal_wrong_exit:consume({ version = 1, sequence = 1, event = "watch.build.started", command = "watch", invocation_id = "watch-fatal-wrong", data = { generation = 0, trigger = "initial" } }); local fatal_wrong_completed = vim.deepcopy(fatal_completed); fatal_wrong_completed.invocation_id = "watch-fatal-wrong"; fatal_wrong_exit:consume(fatal_wrong_completed); fatal_wrong_exit:consume({ version = 1, sequence = 3, event = "watch.stopped", command = "watch", invocation_id = "watch-fatal-wrong", data = { reason = { type = "fatal" }, error = { category = "input", code = "missing_path", operation = "resolve_path" } } }); expect_error(function() fatal_wrong_exit:finish(0) end, "watch_exit_mismatch")
local recovery_boundary = vim.deepcopy(completed.data); recovery_boundary.recovery = { { marker = { encoding = "utf8", value = root }, reason = "stage_cleanup_failed", detail = { type = "io", kind = "other", raw_os_error = 2147483647 } } }; assert_true(watch_protocol.valid_completed(recovery_boundary), "i32 recovery errno boundary was rejected"); recovery_boundary.recovery[1].detail.raw_os_error = protocol.integer("2147483648"); assert_true(not watch_protocol.valid_completed(recovery_boundary), "wide recovery errno was accepted")
local hostile = watch_protocol.new("watch-hostile", root); local hostile_started = vim.deepcopy(started); hostile_started.invocation_id = "watch-hostile"; hostile:consume(hostile_started); expect_error(function() hostile:consume({ version = 1, sequence = 1, event = "watch.build.started", command = "watch", invocation_id = "watch-hostile", data = { generation = 1, trigger = "initial" } }) end, "invalid_build_started")
local hostile_control = watch_protocol.new("watch-control", root); local control_started = vim.deepcopy(hostile_started); control_started.invocation_id = "watch-control"; hostile_control:consume(control_started); expect_error(function() hostile_control:consume({ version = 1, sequence = 1, event = "watch.control.error", command = "watch", invocation_id = "watch-control", data = { error = { type = "wrong" } } }) end, "invalid_control_error")
