local protocol = require("recite.command_protocol")
local finite = require("recite.finite_protocol")
local process = require("recite.command_process")
local watch = require("recite.watch")
local command_diagnostics = require("recite.command_diagnostics")

vim.notify = function() end
local function fail(message) error("Neovim command lifecycle check: " .. message, 0) end
local function assert_true(value, message) if not value then fail(message) end end
local function wait_for(predicate, message) assert_true(vim.wait(10000, predicate, 25), message) end
local root = vim.env.RECITE_TEST_PROJECT

local fake_argv
local fake_system = function(argv, opts, on_exit)
  fake_argv = argv
  local child = { killed = false }
  function child:write() end
  function child:kill(signal) self.killed = signal end
  vim.schedule(function()
    opts.stdout(nil, '{"version":1,"sequence":0,"event":"command.started","command":"validate","invocation_id":"fake"}\n')
    opts.stdout(nil, '{"version":1,"sequence":1,"event":"command.result","command":"validate","invocation_id":"fake","status":"success","exit_code":0,"data":{"diagnostics":[]}}\n')
    on_exit({ code = 0, signal = 0 })
  end)
  return child
end
local fake_done
process.start_finite({ argv = { "recite", "validate" }, command = "validate", invocation_id = "fake", system = fake_system, on_error = function(error) fail("fake process unexpectedly failed: " .. protocol.error_message(error)) end, on_result = function(result) fake_done = result end, on_close = function() end })
wait_for(function() return fake_done ~= nil end, "fake finite process did not complete")
assert_true(fake_argv[1] == "recite" and fake_argv[2] == "validate", "finite process did not preserve argv")

local stderr_error, stderr_error_count, stderr_close_count = nil, 0, 0
process.start_finite({ argv = { "recite", "validate" }, command = "validate", invocation_id = "fake-stderr", system = function(_, opts)
  local child = {}; function child:kill() end; function child:write() end
  vim.schedule(function() opts.stderr(nil, "leak"); opts.stderr(nil, "leak-again") end); return child
end, on_error = function(error) stderr_error = error; stderr_error_count = (stderr_error_count or 0) + 1 end, on_result = function() end, on_close = function() stderr_close_count = (stderr_close_count or 0) + 1 end })
wait_for(function() return stderr_error ~= nil end, "stderr leakage was not rejected")
assert_true(stderr_error.code == "stderr_output", "stderr leakage used the wrong error"); vim.wait(100)
assert_true(stderr_error_count == 1 and stderr_close_count == 0, "stderr failure callback was not settled exactly once before close")

local content_result
process.start_finite({ argv = { "recite", "validate" }, command = "validate", invocation_id = "content", system = function(_, opts, on_exit)
  local child = {}; function child:kill() end; function child:write() end
  vim.schedule(function() opts.stdout(nil, '{"version":1,"sequence":0,"event":"command.started","command":"validate","invocation_id":"content"}\n'); opts.stdout(nil, '{"version":1,"sequence":1,"event":"command.result","command":"validate","invocation_id":"content","status":"content_diagnostics","exit_code":1,"data":{"diagnostics":[]}}\n'); on_exit({ code = 1, signal = 0 }) end); return child
end, on_error = function(error) fail("content diagnostics unexpectedly failed: " .. protocol.error_message(error)) end, on_result = function(result) content_result = result end, on_close = function() end })
wait_for(function() return content_result ~= nil end, "content diagnostic result did not complete")
assert_true(content_result.terminal.status == "content_diagnostics", "content diagnostics lost its terminal status")

local callback_result_error, callback_close_count, callback_failure_count
process.start_finite({ argv = { "recite", "validate" }, command = "validate", invocation_id = "callback-result", system = function(_, opts, on_exit)
  local child = {}; function child:kill() end; function child:write() end
  vim.schedule(function() opts.stdout(nil, '{"version":1,"sequence":0,"event":"command.started","command":"validate","invocation_id":"callback-result"}\n'); opts.stdout(nil, '{"version":1,"sequence":1,"event":"command.result","command":"validate","invocation_id":"callback-result","status":"success","exit_code":0,"data":{"diagnostics":[]}}\n'); on_exit({ code = 0, signal = 0 }) end); return child
end, on_callback_failed = function() callback_failure_count = (callback_failure_count or 0) + 1 end, on_error = function(error) callback_result_error = error end, on_result = function() error("intentional result callback failure") end, on_close = function() callback_close_count = (callback_close_count or 0) + 1 end })
wait_for(function() return callback_result_error ~= nil and callback_close_count == 1 end, "throwing result callback did not settle and close")
assert_true(callback_result_error.code == "callback_failed" and callback_failure_count == 1, "result callback failure was not typed exactly once")
local callback_error, callback_error_close, callback_error_failure, callback_error_exit
process.start_finite({ argv = { "recite", "validate" }, command = "validate", invocation_id = "callback-error", system = function(_, opts, on_exit)
  callback_error_exit = on_exit; local child = {}; function child:kill() end; function child:write() end; vim.schedule(function() opts.stderr(nil, "leak") end); return child
end, term_timeout_ms = 20, kill_timeout_ms = 20, on_callback_failed = function() callback_error_failure = (callback_error_failure or 0) + 1 end, on_error = function(error) callback_error = error; error("intentional error callback failure") end, on_result = function() end, on_close = function() callback_error_close = (callback_error_close or 0) + 1 end })
wait_for(function() return callback_error ~= nil end, "throwing error callback did not run"); callback_error_exit({ code = 1, signal = 9 }); wait_for(function() return callback_error_close == 1 end, "throwing error callback did not finalize close"); assert_true(callback_error_failure == 1, "error callback failure was not reported exactly once")

local typed_error = { version = 1, sequence = 0, event = "command.started", command = "validate", invocation_id = "operational" }
local typed_terminal = { version = 1, sequence = 1, event = "command.error", command = "validate", invocation_id = "operational", status = "failure", exit_code = 1, error = { category = "input", code = "missing_path", operation = "validate" } }
assert_true(finite.parse({ typed_error, typed_terminal }, "validate", "operational", 1).terminal.event == "command.error", "typed operational error was not retained")
local spawn_error
process.start_finite({ argv = { "missing-recite", "validate" }, command = "validate", invocation_id = "spawn", system = function() error("spawn refused") end, on_error = function(error) spawn_error = error end, on_result = function() end, on_close = function() end })
wait_for(function() return spawn_error ~= nil end, "spawn failure was not reported"); assert_true(spawn_error.code == "spawn", "spawn failure used the wrong protocol code")

local commands_adapter = require("recite.commands").new({ config = { binary = vim.env.RECITE_CLI, max_finite_bytes = 1024 * 1024 }, root_dir = function() return root end })
local command_error_count = 0
local command_error_system = function(_, opts, on_exit)
  local child = {}; function child:kill() end; function child:write() end
  vim.schedule(function() opts.stdout(nil, '{"version":1,"sequence":0,"event":"command.started","command":"validate","invocation_id":"command-error"}\n'); opts.stdout(nil, '{"version":1,"sequence":1,"event":"command.error","command":"validate","invocation_id":"command-error","status":"failure","exit_code":1,"error":{"category":"input","code":"missing_path","operation":"validate"}}\n'); on_exit({ code = 1, signal = 0 }) end); return child
end
commands_adapter.validate({ project_root = root, paths = { root .. "/core_language_spike.recite" }, invocation_id = "command-error", config = { system = command_error_system }, on_error = function() command_error_count = command_error_count + 1 end })
wait_for(function() return command_error_count == 1 end, "command.error did not settle on_error"); vim.wait(100); assert_true(command_error_count == 1, "command.error settled the adapter callback more than once")

-- A start handle is not ownership: a spawn failure or an exit before the
-- validated watch.started record must leave finite diagnostics untouched.
local seeded_namespace, seeded_result = nil, false
local original_seed_replace = command_diagnostics.replace
command_diagnostics.replace = function(namespace, records, ...)
  if records and #records > 0 and not seeded_namespace then seeded_namespace = namespace end
  return original_seed_replace(namespace, records, ...)
end
local seeded_record = {
  version = 1, code = "RECITE_X001", severity = "error",
  span = { file = "core_language_spike.recite", start = { line = 1, column = 1 }, ["end"] = { line = 1, column = 1 } },
  presentation = { id = "diagnostic-parse-001", arguments = vim.empty_dict() }, related = {},
  help = vim.NIL, explanation = vim.NIL, compatibility_message = "seeded",
}
local seed_system = function(_, opts, on_exit)
  local child = {}; function child:kill() end; function child:write() end
  vim.schedule(function()
    opts.stdout(nil, '{"version":1,"sequence":0,"event":"command.started","command":"validate","invocation_id":"seed-diagnostics"}\n')
    opts.stdout(nil, vim.json.encode({ version = 1, sequence = 1, event = "command.result", command = "validate", invocation_id = "seed-diagnostics", status = "success", exit_code = 0, data = { diagnostics = { seeded_record } } }) .. "\n")
    on_exit({ code = 0, signal = 0 })
  end); return child
end
commands_adapter.validate({ project_root = root, paths = { root .. "/core_language_spike.recite" }, invocation_id = "seed-diagnostics", config = { system = seed_system }, on_result = function() seeded_result = true end, on_error = function(error) fail("diagnostic seed failed: " .. protocol.error_message(error)) end })
wait_for(function() return seeded_result and seeded_namespace ~= nil end, "finite diagnostics were not seeded")
local seeded_bufnr = vim.fn.bufnr(vim.fn.fnamemodify(root .. "/core_language_spike.recite", ":p"))
assert_true(#vim.diagnostic.get(seeded_bufnr, { namespace = seeded_namespace }) == 1, "finite diagnostic seed was not projected")
local spawn_failed = commands_adapter.watch_start({ project_root = root, invocation_id = "watch-spawn-failed", system = function() error("spawn refused before watch.started") end })
assert_true(spawn_failed ~= nil, "spawn-failed watch did not return its transport handle")
wait_for(function() return commands_adapter.watch_active() == nil end, "spawn-failed watch did not close")
assert_true(#vim.diagnostic.get(seeded_bufnr, { namespace = seeded_namespace }) == 1, "failed watch start cleared finite diagnostics")
local exit_before_started = commands_adapter.watch_start({ project_root = root, invocation_id = "watch-exit-before-start", system = function(_, _, on_exit)
  local child = {}; function child:write() end; function child:kill() end
  vim.schedule(function() on_exit({ code = 1, signal = 0 }) end); return child
end })
assert_true(exit_before_started ~= nil, "pre-start exit watch did not return its transport handle")
wait_for(function() return commands_adapter.watch_active() == nil end, "pre-start exit watch did not close")
assert_true(#vim.diagnostic.get(seeded_bufnr, { namespace = seeded_namespace }) == 1, "pre-start exit cleared finite diagnostics")
command_diagnostics.replace = original_seed_replace

local malformed_error, malformed_close = nil, 0
commands_adapter.validate({ project_root = root, paths = { root .. "/core_language_spike.recite" }, invocation_id = "malformed-adapter", config = { system = function(_, opts, on_exit)
  local child = {}; function child:kill() end; function child:write() end
  vim.schedule(function() opts.stdout(nil, "not-json\n"); on_exit({ code = 1, signal = 0 }) end); return child
end }, on_error = function(error) malformed_error = error end, on_result = function() end })
wait_for(function() return malformed_error ~= nil end, "adapter malformed output did not settle on_error")
assert_true(malformed_error.code == "invalid_json", "adapter malformed output used the wrong typed error")
local stderr_adapter_error
commands_adapter.validate({ project_root = root, paths = { root .. "/core_language_spike.recite" }, invocation_id = "stderr-adapter", config = { system = function(_, opts, on_exit)
  local child = {}; function child:kill() end; function child:write() end
  vim.schedule(function() opts.stderr(nil, "stderr"); on_exit({ code = 1, signal = 0 }) end); return child
end }, on_error = function(error) stderr_adapter_error = error end, on_result = function() end })
wait_for(function() return stderr_adapter_error ~= nil end, "adapter stderr/fast-exit did not settle on_error")
assert_true(stderr_adapter_error.code == "stderr_output", "adapter stderr/fast-exit used the wrong typed error")

local reentrant_first_result, reentrant_first_error, reentrant_second_result, reentrant_started = nil, nil, nil, false
local reentrant_system = function(argv, opts, on_exit)
  local invocation = argv[#argv]; local child = {}; function child:write() end; function child:kill() end
  vim.schedule(function()
    opts.stdout(nil, string.format('{"version":1,"sequence":0,"event":"command.started","command":"validate","invocation_id":"%s"}\n', invocation))
    opts.stdout(nil, string.format('{"version":1,"sequence":1,"event":"command.result","command":"validate","invocation_id":"%s","status":"success","exit_code":0,"data":{"diagnostics":[]}}\n', invocation))
    on_exit({ code = 0, signal = 0 })
  end); return child
end
commands_adapter.validate({ project_root = root, paths = { root .. "/core_language_spike.recite" }, invocation_id = "reentrant-first", config = { system = reentrant_system }, on_result = function()
  reentrant_first_result = true
  if not reentrant_started then
    reentrant_started = true
    commands_adapter.validate({ project_root = root, paths = { root .. "/core_language_spike.recite" }, invocation_id = "reentrant-second", config = { system = reentrant_system }, on_result = function() reentrant_second_result = true end, on_error = function(error) fail("reentrant second invocation failed: " .. protocol.error_message(error)) end })
  end
end, on_error = function(error) reentrant_first_error = error end })
wait_for(function() return reentrant_first_result and reentrant_second_result end, "reentrant result callback did not settle both invocations")
assert_true(reentrant_first_error == nil, "reentrant result callback caused a superseded error on the settled invocation")

local stale_callbacks, stale_cwd, stale_spawn_count, stale_live = {}, nil, 0, 0
local stale_system = function(argv, opts, on_exit)
  stale_cwd = opts.cwd; local invocation = argv[#argv]; stale_spawn_count = stale_spawn_count + 1; stale_live = stale_live + 1; local child = {}; function child:write() end; function child:kill() end
  stale_callbacks[invocation] = function() opts.stdout(nil, string.format('{"version":1,"sequence":0,"event":"command.started","command":"validate","invocation_id":"%s"}\n', invocation)); opts.stdout(nil, string.format('{"version":1,"sequence":1,"event":"command.result","command":"validate","invocation_id":"%s","status":"success","exit_code":0,"data":{"diagnostics":[]}}\n', invocation)); on_exit({ code = 0, signal = 0 }); stale_live = stale_live - 1 end; return child
end
local stale_first, stale_second, stale_first_failure, stale_second_failure
local stale_source = root .. "/core_language_spike.recite"
local first_invocation = commands_adapter.validate({ project_root = root, paths = { stale_source }, config = { system = stale_system }, on_result = function() stale_first = true end, on_error = function(error) stale_first_failure = error end })
local second_invocation = commands_adapter.validate({ project_root = root, paths = { stale_source }, config = { system = stale_system }, on_result = function() stale_second = true end, on_error = function(error) stale_second_failure = error end })
assert_true(first_invocation and second_invocation and first_invocation.invocation_id ~= second_invocation.invocation_id, "generation test did not create two invocations"); assert_true(stale_cwd == root, "finite source command did not use project root cwd"); wait_for(function() return stale_first_failure ~= nil end, "superseded finite invocation did not settle"); assert_true(stale_first_failure.code == "superseded" and stale_spawn_count == 1 and stale_live == 1, "replacement overlapped the predecessor or used the wrong settlement"); assert_true(stale_second == nil, "replacement spawned before predecessor close")
stale_callbacks[first_invocation.invocation_id](); wait_for(function() return stale_spawn_count == 2 end, "replacement did not wait for predecessor close"); assert_true(stale_live == 1, "replacement exceeded one live child"); stale_callbacks[second_invocation.invocation_id](); wait_for(function() return stale_second or stale_second_failure end, "replacement invocation did not complete"); assert_true(stale_second ~= nil and stale_second_failure == nil, "replacement invocation failed"); assert_true(not stale_first, "late finite result escaped generation fencing")

local reconfigure_old_emit, reconfigure_old_exit, reconfigure_old_kill
local reconfigure_old_result, reconfigure_old_error, reconfigure_old_error_count = nil, nil, 0
local reconfigure_new_spawned, reconfigure_new_result = false, nil
local reconfigure_old_system = function(argv, opts, on_exit)
  local invocation = argv[#argv]
  local child = {}
  function child:write() end
  function child:kill(signal) reconfigure_old_kill = signal end
  reconfigure_old_emit = function()
    opts.stdout(nil, string.format('{"version":1,"sequence":0,"event":"command.started","command":"validate","invocation_id":"%s"}\n', invocation))
    opts.stdout(nil, string.format('{"version":1,"sequence":1,"event":"command.result","command":"validate","invocation_id":"%s","status":"success","exit_code":0,"data":{"diagnostics":[]}}\n', invocation))
  end
  reconfigure_old_exit = on_exit
  return child
end
local reconfigure_new_system = function(argv, opts, on_exit)
  reconfigure_new_spawned = true
  local invocation = argv[#argv]
  local child = {}
  function child:write() end
  function child:kill() end
  vim.schedule(function()
    opts.stdout(nil, string.format('{"version":1,"sequence":0,"event":"command.started","command":"validate","invocation_id":"%s"}\n', invocation))
    opts.stdout(nil, string.format('{"version":1,"sequence":1,"event":"command.result","command":"validate","invocation_id":"%s","status":"success","exit_code":0,"data":{"diagnostics":[]}}\n', invocation))
    on_exit({ code = 0, signal = 0 })
  end)
  return child
end
commands_adapter.configure({ system = reconfigure_old_system, finite_stop_timeout_ms = 20, finite_kill_timeout_ms = 20 })
commands_adapter.validate({ project_root = root, paths = { stale_source }, invocation_id = "reconfigure-old", on_result = function() reconfigure_old_result = true end, on_error = function(error) reconfigure_old_error = error; reconfigure_old_error_count = reconfigure_old_error_count + 1 end })
commands_adapter.configure({ system = reconfigure_new_system, finite_stop_timeout_ms = 200, finite_kill_timeout_ms = 200 })
assert_true(reconfigure_old_kill == "sigterm", "finite reconfiguration did not cancel the old process")
assert_true(reconfigure_old_error and reconfigure_old_error.code == "cancelled" and reconfigure_old_error_count == 1, "finite reconfiguration did not settle the old callback exactly once")
commands_adapter.validate({ project_root = root, paths = { stale_source }, invocation_id = "reconfigure-new", on_result = function(result) reconfigure_new_result = result end, on_error = function(error) fail("reconfigured finite invocation failed: " .. protocol.error_message(error)) end })
assert_true(not reconfigure_new_spawned, "finite replacement launched before the old process closed")
reconfigure_old_emit()
reconfigure_old_exit({ code = 0, signal = 15 })
wait_for(function() return reconfigure_new_spawned end, "finite replacement did not launch after the old process closed")
wait_for(function() return reconfigure_new_result ~= nil end, "reconfigured finite invocation did not complete")
assert_true(reconfigure_old_result == nil, "cancelled finite result escaped reconfiguration fencing")

local no_op_child, no_op_exit, no_op_killed, no_op_error
local no_op_system = function(_, _, on_exit)
  no_op_child = {}
  function no_op_child:write() end
  function no_op_child:kill(signal) no_op_killed = signal end
  no_op_exit = on_exit
  return no_op_child
end
commands_adapter.configure({ system = no_op_system })
commands_adapter.validate({ project_root = root, paths = { stale_source }, invocation_id = "reconfigure-no-op", on_result = function() end, on_error = function(error) no_op_error = error end })
assert_true(commands_adapter.configure({ system = no_op_system }) == false, "unchanged command configuration churned the adapter")
assert_true(no_op_killed == nil, "unchanged command configuration cancelled a finite process")
no_op_exit({ code = 0, signal = 0 })
wait_for(function() return no_op_error ~= nil end, "no-op configuration test process did not close")

local fake_watch_ready, fake_watch_child, fake_watch_failure = false, nil, nil
local fake_watch_cancel_requested, fake_watch_stopped = false, false
local fake_watch_system = function(_, opts, on_exit)
  fake_watch_child = {}; function fake_watch_child:write(data) if not data then return end; local cancel = { version = 1, sequence = 4, event = "watch.cancel.requested", command = "watch", invocation_id = "fake-watch", data = { cancellation = { type = "user" } } }; local stopped = { version = 1, sequence = 5, event = "watch.stopped", command = "watch", invocation_id = "fake-watch", data = { reason = { type = "cancelled" } } }; vim.schedule(function() local line = vim.json.encode(cancel) .. "\n"; opts.stdout(nil, line:sub(1, 7)); opts.stdout(nil, line:sub(8)); opts.stdout(nil, vim.json.encode(stopped) .. "\n"); on_exit({ code = 0, signal = 0 }) end) end; function fake_watch_child:kill(signal) self.signal = signal end
  vim.schedule(function() local first = { version = 1, sequence = 0, event = "watch.started", command = "watch", invocation_id = "fake-watch", data = { project_root = { encoding = "utf8", value = root } } }; local build = { version = 1, sequence = 1, event = "watch.build.started", command = "watch", invocation_id = "fake-watch", data = { generation = 0, trigger = "initial" } }; local done_line = '{"version":1,"sequence":2,"event":"watch.build.completed","command":"watch","invocation_id":"fake-watch","data":{"generation":0,"snapshot_generation":null,"status":"succeeded","outcome":{"type":"fresh"},"inputs":[],"diagnostics":[],"artifacts":[],"freshness":{"type":"fresh"},"publication":{"type":"not_attempted","reason":"no_candidates"},"recovery":[],"restart_guidance":{"type":"host_policy_required","decision":"unspecified"}}}\n'; local waiting = { version = 1, sequence = 3, event = "watch.waiting", command = "watch", invocation_id = "fake-watch", data = vim.empty_dict() }; opts.stdout(nil, vim.json.encode(first) .. "\n"); opts.stdout(nil, vim.json.encode(build) .. "\n"); opts.stdout(nil, done_line); opts.stdout(nil, vim.json.encode(waiting) .. "\n") end); return fake_watch_child
end
local fake_watch = watch.new({ config = { binary = vim.env.RECITE_CLI, max_watch_bytes = 1024 * 1024, watch_stop_timeout_ms = 100, watch_force_kill_delay_ms = 20, watch_teardown_timeout_ms = 100 }, notify = function(id, arguments) if id == "neovim-command-watch-status" then fake_watch_ready = true; local detail = arguments and arguments.detail or ""; fake_watch_cancel_requested = fake_watch_cancel_requested or detail:find("user", 1, true) ~= nil; fake_watch_stopped = fake_watch_stopped or detail:find("cancelled", 1, true) ~= nil elseif id == "neovim-command-protocol-failure" then fake_watch_failure = arguments and arguments.detail or id end end, clear_diagnostics = function() end, replace_diagnostics = function() end })
assert_true(fake_watch.start({ project_root = root, invocation_id = "fake-watch", system = fake_watch_system }) ~= nil, "fake watch did not start"); wait_for(function() return fake_watch_ready end, "fake watch did not reach idle waiting"); assert_true(fake_watch.stop(), "fake watch did not accept cooperative cancellation"); wait_for(function() return fake_watch.active() == nil end, "fake watch did not retire after cancellation")
assert_true(fake_watch_cancel_requested and fake_watch_stopped and fake_watch_failure == nil, "queued cancellation/stopped records were discarded before stream close")

local original_command_replace, command_replace_calls = command_diagnostics.replace, 0
command_diagnostics.replace = function(...) command_replace_calls = command_replace_calls + 1; return original_command_replace(...) end
local late_finite_callback, late_finite_result
local late_finite_system = function(argv, opts, on_exit)
  local invocation = argv[#argv]; local child = {}; function child:write() end; function child:kill() end
  late_finite_callback = function() opts.stdout(nil, string.format('{"version":1,"sequence":0,"event":"command.started","command":"validate","invocation_id":"%s"}\n', invocation)); opts.stdout(nil, string.format('{"version":1,"sequence":1,"event":"command.result","command":"validate","invocation_id":"%s","status":"success","exit_code":0,"data":{"diagnostics":[]}}\n', invocation)); on_exit({ code = 0, signal = 0 }); end; return child
end
commands_adapter.validate({ project_root = root, paths = { stale_source }, config = { system = late_finite_system }, on_result = function(result) late_finite_result = result end, on_error = function(error) fail("late finite failed: " .. protocol.error_message(error)) end })
assert_true(commands_adapter.watch_start({ project_root = root, invocation_id = "fake-watch", system = fake_watch_system }) ~= nil, "finite-to-watch ownership test did not start watch"); wait_for(function() return command_replace_calls > 0 end, "watch did not publish initial diagnostics"); local replace_before_late = command_replace_calls; late_finite_callback(); wait_for(function() return late_finite_result ~= nil end, "finite result did not settle while watch was active"); assert_true(command_replace_calls == replace_before_late, "finite diagnostics overwrote watch-owned diagnostics"); commands_adapter.watch_stop(); wait_for(function() return commands_adapter.watch_active() == nil end, "watch did not stop"); command_diagnostics.replace = original_command_replace

local function reconfigure_watch_completed(invocation, sequence)
  return string.format('{"version":1,"sequence":%d,"event":"watch.build.completed","command":"watch","invocation_id":"%s","data":{"generation":0,"snapshot_generation":null,"status":"succeeded","outcome":{"type":"fresh"},"inputs":[],"diagnostics":[],"artifacts":[],"freshness":{"type":"fresh"},"publication":{"type":"not_attempted","reason":"no_candidates"},"recovery":[],"restart_guidance":{"type":"host_policy_required","decision":"unspecified"}}}\n', sequence, invocation)
end
local reconfigure_watch_old_emit, reconfigure_watch_old_exit, reconfigure_watch_old_child
local reconfigure_watch_ready, reconfigure_watch_signals = false, {}
local reconfigure_watch_old_system = function(argv, opts, on_exit)
  local invocation = argv[#argv - 1]
  reconfigure_watch_old_child = {}
  function reconfigure_watch_old_child:write() end
  function reconfigure_watch_old_child:kill(signal) reconfigure_watch_signals[#reconfigure_watch_signals + 1] = signal end
  reconfigure_watch_old_emit = function()
    opts.stdout(nil, reconfigure_watch_completed(invocation, 4))
  end
  reconfigure_watch_old_exit = on_exit
  vim.schedule(function()
    opts.stdout(nil, vim.json.encode({ version = 1, sequence = 0, event = "watch.started", command = "watch", invocation_id = invocation, data = { project_root = { encoding = "utf8", value = root } } }) .. "\n")
    opts.stdout(nil, vim.json.encode({ version = 1, sequence = 1, event = "watch.build.started", command = "watch", invocation_id = invocation, data = { generation = 0, trigger = "initial" } }) .. "\n")
    opts.stdout(nil, reconfigure_watch_completed(invocation, 2))
    opts.stdout(nil, vim.json.encode({ version = 1, sequence = 3, event = "watch.waiting", command = "watch", invocation_id = invocation, data = vim.empty_dict() }) .. "\n")
    reconfigure_watch_ready = true
  end)
  return reconfigure_watch_old_child
end
local reconfigure_watch_new_spawned = false
local reconfigure_watch_new_system = function(argv, opts, on_exit)
  local invocation = argv[#argv - 1]
  reconfigure_watch_new_spawned = true
  local child = {}
  function child:write(data)
    if not data then return end
    opts.stdout(nil, vim.json.encode({ version = 1, sequence = 1, event = "watch.cancel.requested", command = "watch", invocation_id = invocation, data = { cancellation = { type = "user" } } }) .. "\n")
    opts.stdout(nil, vim.json.encode({ version = 1, sequence = 2, event = "watch.stopped", command = "watch", invocation_id = invocation, data = { reason = { type = "cancelled" } } }) .. "\n")
    on_exit({ code = 0, signal = 0 })
  end
  function child:kill() end
  vim.schedule(function()
    opts.stdout(nil, vim.json.encode({ version = 1, sequence = 0, event = "watch.started", command = "watch", invocation_id = invocation, data = { project_root = { encoding = "utf8", value = root } } }) .. "\n")
  end)
  return child
end
local reconfigure_watch_replace_calls = 0
local original_reconfigure_watch_replace = command_diagnostics.replace
command_diagnostics.replace = function(...)
  reconfigure_watch_replace_calls = reconfigure_watch_replace_calls + 1
  return original_reconfigure_watch_replace(...)
end
commands_adapter.configure({ system = reconfigure_watch_old_system, watch_cancel_grace_ms = 10, watch_stop_timeout_ms = 20, watch_force_kill_delay_ms = 10, watch_teardown_timeout_ms = 20 })
assert_true(commands_adapter.watch_start({ project_root = root, invocation_id = "reconfigure-watch-old" }) ~= nil, "reconfiguration watch did not start")
wait_for(function() return reconfigure_watch_ready end, "reconfiguration watch did not reach its first build")
assert_true(reconfigure_watch_replace_calls == 1, "reconfiguration watch did not publish its initial diagnostics")
commands_adapter.configure({ system = reconfigure_watch_new_system, watch_cancel_grace_ms = 200, watch_stop_timeout_ms = 200, watch_force_kill_delay_ms = 200, watch_teardown_timeout_ms = 200 })
assert_true(commands_adapter.watch_active() ~= nil, "watch reconfiguration discarded ownership before close")
reconfigure_watch_old_emit()
vim.wait(100)
assert_true(reconfigure_watch_replace_calls == 1, "fenced watch published stale diagnostics")
wait_for(function() return reconfigure_watch_signals[1] == "sigterm" and reconfigure_watch_signals[2] == "sigkill" end, "watch reconfiguration did not use the old bounded shutdown timing")
assert_true(commands_adapter.watch_active() ~= nil, "watch reconfiguration released its tombstone before close")
reconfigure_watch_old_exit({ code = 0, signal = 9 })
wait_for(function() return commands_adapter.watch_active() == nil end, "watch reconfiguration did not release ownership after close")
assert_true(commands_adapter.watch_start({ project_root = root, invocation_id = "reconfigure-watch-new" }) ~= nil, "watch did not become reusable after reconfiguration close")
wait_for(function() return reconfigure_watch_new_spawned end, "watch did not use the replacement configuration")
commands_adapter.watch_stop()
wait_for(function() return commands_adapter.watch_active() == nil end, "replacement watch did not close")
command_diagnostics.replace = original_reconfigure_watch_replace

local hung_child, hung_on_exit, hung_failure, hung_signals = nil, nil, nil, {}
local hung_watch = watch.new({ config = { binary = vim.env.RECITE_CLI, watch_stop_timeout_ms = 20, watch_force_kill_delay_ms = 20, watch_teardown_timeout_ms = 20 }, notify = function(id) if id == "neovim-command-protocol-failure" then hung_failure = true end end, clear_diagnostics = function() end, replace_diagnostics = function() end })
local hung_system = function(_, _, on_exit) hung_child = {}; hung_on_exit = on_exit; hung_signals = {}; function hung_child:write() end; function hung_child:kill(signal) self.signal = signal; hung_signals[#hung_signals + 1] = signal end; return hung_child end
assert_true(hung_watch.start({ project_root = root, invocation_id = "hung-watch", system = hung_system }) ~= nil, "hung watch did not start"); assert_true(hung_watch.stop(), "hung watch did not begin recovery"); wait_for(function() return hung_child.signal == "sigkill" and hung_failure and hung_watch.active() ~= nil end, "hung watch did not retain its tombstone after TERM/KILL"); assert_true(hung_watch.start({ project_root = root, invocation_id = "blocked-watch", system = hung_system }) == nil, "hung watch allowed replacement before on_exit"); hung_on_exit({ code = 0, signal = 9 }); wait_for(function() return hung_watch.active() == nil end, "hung watch did not retire after actual on_exit")
assert_true(hung_signals[1] == "sigterm" and hung_signals[2] == "sigkill", "watch recovery did not signal TERM before KILL")

local recite = require("recite")
recite.setup({ lsp = { autostart = false }, commands = { binary = vim.env.RECITE_CLI } })
local source = root .. "/core_language_spike.recite"
local captured_validate, captured_compile
local original_validate, original_compile = recite.validate, recite.compile
recite.validate = function(options) captured_validate = options end; recite.compile = function(options) captured_compile = options end
local spaced_path = root .. "/source with spaces.recite"; vim.cmd("ReciteValidate " .. vim.fn.fnameescape(spaced_path)); assert_true(captured_validate and captured_validate.paths[1] == spaced_path, "escaped-space user command path was not preserved")
local output_only = root .. "/output with spaces.recitec"; vim.cmd("ReciteCompile " .. vim.fn.fnameescape(output_only)); assert_true(captured_compile and captured_compile.output == output_only and captured_compile.paths == nil, "compile output-only user command grammar did not preserve the default input selection"); recite.validate, recite.compile = original_validate, original_compile
local output, extract, fixture = root .. "/neovim-command-test.recitec", root .. "/neovim-command-test.pot", root .. "/neovim-command-test.fixture.toml"
local fixture_handle = assert(io.open(fixture, "wb")); fixture_handle:write('[conditions]\n"trusts(player)" = true\n\n[choices]\n637b1854a7f3ed42f045 = "b84cc9fa241a33bcdf05"\n\n[effects]\nauto_ack_blocking = true\n'); fixture_handle:close()
local results = {}
local function invoke(name, options) options.on_result = function(result) results[name] = result end; options.on_error = function(error) fail(name .. " failed: " .. protocol.error_message(error)) end; recite.commands[name](options); wait_for(function() return results[name] ~= nil end, "real " .. name .. " did not complete"); assert_true(results[name].terminal.event == "command.result", name .. " did not return a result") end
invoke("validate", { project_root = root, paths = { source } }); invoke("compile", { project_root = root, paths = { source }, output = output }); invoke("extract", { project_root = root, paths = { source }, output = extract }); invoke("run", { asset = output, block = "start", fixture = fixture }); invoke("trace", { asset = output, block = "start", fixture = fixture })
local watch_session = recite.watch_start({ project_root = root }); assert_true(watch_session ~= nil, "real watch did not start"); wait_for(function() return recite.watch_active() ~= nil end, "real watch ownership was not retained"); assert_true(recite.watch_stop(), "real watch did not accept cancellation"); wait_for(function() return recite.watch_active() == nil end, "real watch did not stop"); os.remove(output); os.remove(extract); os.remove(fixture)

local exit_watch_child, exit_watch_signal, exit_watch_signals = nil, nil, {}
local exit_finite_child, exit_finite_signals = nil, {}
local exit_watch_system = function(argv, _, on_exit)
  if argv[2] ~= "watch" then
    local child = {}; exit_finite_child = child; exit_finite_signals = {}
    function child:write() end
    function child:kill(signal) exit_finite_signals[#exit_finite_signals + 1] = signal; if signal == "sigkill" then on_exit({ code = 9, signal = 9 }) end end
    return child
  end
  local child = {}; exit_watch_child = child
  function child:write() end
  function child:kill(signal) exit_watch_signal = signal; exit_watch_signals[#exit_watch_signals + 1] = signal; if signal == "sigkill" then on_exit({ code = 9, signal = 9 }) end end
  return child
end
recite.commands.configure({ system = exit_watch_system, watch_stop_timeout_ms = 20, watch_teardown_timeout_ms = 20, finite_stop_timeout_ms = 20, finite_kill_timeout_ms = 20 }); assert_true(recite.commands.validate({ project_root = root, paths = { source }, on_result = function() end, on_error = function() end }) ~= nil, "VimLeavePre finite session did not start"); assert_true(recite.watch_start({ project_root = root, invocation_id = "leave-watch" }) ~= nil, "VimLeavePre active watch did not start"); vim.cmd("doautocmd <nomodeline> VimLeavePre"); assert_true(exit_watch_signal == "sigkill" and exit_watch_signals[1] == "sigterm" and exit_watch_signals[2] == "sigkill" and exit_watch_child ~= nil and exit_finite_child ~= nil and exit_finite_signals[1] == "sigterm" and exit_finite_signals[2] == "sigkill" and recite.watch_active() == nil, "VimLeavePre did not synchronously drain and kill active watch and finite sessions")
vim.cmd("doautocmd <nomodeline> VimLeavePre"); assert_true(recite.watch_start({ project_root = root }) == nil, "VimLeavePre did not dispose the watch adapter")
