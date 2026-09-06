-- Installed-host evidence for the narrow Milestone 4 keyboard workflow.
--
-- This drives Neovim's real command-line input path with feedkeys.  Recite
-- deliberately installs no default mappings, so the documented `:` commands
-- are the portable keyboard entry point.  The shell harness supplies real
-- recite-lsp and recite binaries and checks the process group after VimLeavePre.
local function fail(message)
  error("Neovim installed-host workflow: " .. message, 0)
end

local function assert_true(value, message)
  if not value then
    fail(message)
  end
end

local function wait_for(predicate, message)
  assert_true(vim.wait(10000, predicate, 50), message)
end

local function escaped(path)
  return vim.fn.fnameescape(path)
end

local function feed(keys)
  local input = vim.api.nvim_replace_termcodes(keys, true, false, true)
  vim.api.nvim_feedkeys(input, "xt", false)
end

local function feed_command(command)
  feed(":" .. command .. "<CR>")
end

local function has_notification(notifications, needle)
  for _, notification in ipairs(notifications) do
    if notification.message:find(needle, 1, true) then
      return notification
    end
  end
end

local function lsp_client(bufnr)
  local clients = vim.lsp.get_clients({ bufnr = bufnr or 0, name = "recite-lsp" })
  return clients[1]
end

local project = vim.env.RECITE_TEST_PROJECT
local valid = project .. "/core_language_spike.recite"
local invalid_project = vim.env.RECITE_INVALID_PROJECT
local invalid = invalid_project .. "/invalid.recite"
local runtime_asset = project .. "/build/dialogue.recitec"
local runtime_fixture = project .. "/runtime-fixture.toml"

assert_true(type(project) == "string" and type(invalid_project) == "string",
  "host workflow fixture paths were not supplied")

-- Notifications are the host's text-and-severity status surface.  Capturing
-- both fields proves that the workflow does not depend on colour alone.
local notifications = {}
vim.notify = function(message, level)
  notifications[#notifications + 1] = { message = tostring(message), level = level }
end

vim.cmd("filetype on")
feed_command("edit " .. escaped(invalid))
assert_true(vim.bo.filetype == "recite", ".recite did not activate through a keyboard edit")
wait_for(function()
  local client = lsp_client()
  return client ~= nil and client.initialized and #vim.diagnostic.get(0) > 0
end, "installed host did not attach recite-lsp and publish diagnostics")

local initial_cursor = vim.api.nvim_win_get_cursor(0)
-- Neovim's built-in ]d mapping is present in both supported hosts.  Driving
-- it through feedkeys exercises the same normal-mode route an author uses and
-- avoids depending on a deprecated Lua helper in newer Neovim releases.
feed("]d")
wait_for(function()
  local cursor = vim.api.nvim_win_get_cursor(0)
  return cursor[1] ~= initial_cursor[1] or cursor[2] ~= initial_cursor[2]
end, "diagnostic navigation was not reachable from the keyboard")
local diagnostic_cursor = vim.api.nvim_win_get_cursor(0)
local diagnostic = vim.diagnostic.get(0)[1]
assert_true(diagnostic ~= nil and type(diagnostic.message) == "string" and diagnostic.message ~= "",
  "diagnostic navigation did not expose a textual diagnostic")
assert_true(diagnostic_cursor[1] == diagnostic.lnum + 1,
  "diagnostic navigation did not move to the reported diagnostic line")

feed_command("edit " .. escaped(valid))
assert_true(vim.bo.filetype == "recite", "valid .recite lost its host filetype")
wait_for(function()
  local client = lsp_client()
  return client ~= nil and client.initialized
end, "installed host did not reattach recite-lsp after keyboard navigation")

local validate_before = #notifications
feed_command("ReciteValidate " .. escaped(valid))
wait_for(function()
  return #notifications > validate_before and has_notification(notifications, "Recite validate") ~= nil
end, "keyboard-reachable ReciteValidate did not present structured command status")
local validate_status = has_notification(notifications, "Recite validate")
assert_true(type(validate_status.level) == "number", "validate status omitted host severity")

local compile_before = #notifications
feed_command("ReciteCompile")
wait_for(function()
  return vim.fn.filereadable(project .. "/build/dialogue.recitec") == 1
    and #notifications > compile_before
    and has_notification(notifications, "Recite compile") ~= nil
end, "keyboard-reachable ReciteCompile did not complete")
assert_true(has_notification(notifications, "Recite compile") ~= nil,
  "compile completion was not textually observable")

-- User commands intentionally expose only host-facing notifications.  Attach
-- callbacks at the public adapter boundary for this evidence lane so the
-- finite command assertions inspect the already-validated structured result,
-- rather than scraping notification text or duplicating CLI semantics.
local recite = require("recite")
local fixture_handle = assert(io.open(runtime_fixture, "wb"))
fixture_handle:write("# The work block has no choices or conditions.\n")
fixture_handle:close()

local function structured_user_command(user_command, adapter_name, arguments)
  local original = recite[adapter_name]
  local result, command_error
  recite[adapter_name] = function(options)
    options = options or {}
    options.on_result = function(value) result = value end
    options.on_error = function(value) command_error = value end
    return original(options)
  end
  feed_command(user_command .. " " .. arguments)
  wait_for(function()
    return result ~= nil or command_error ~= nil
  end, user_command .. " did not return a structured result")
  recite[adapter_name] = original
  assert_true(command_error == nil,
    user_command .. " returned an error: " .. tostring(command_error and command_error.detail or command_error))
  assert_true(result ~= nil and result.terminal ~= nil,
    user_command .. " did not expose a terminal record")
  assert_true(result.terminal.event == "command.result",
    user_command .. " did not expose command.result")
  assert_true(result.terminal.command == adapter_name,
    user_command .. " returned the wrong command")
  assert_true(result.terminal.status == "success" and result.terminal.exit_code == 0,
    user_command .. " did not report structured success")
  assert_true(type(result.terminal.data) == "table",
    user_command .. " did not expose structured data")
  return result
end

local extract_output = project .. "/host-extract.pot"
local extract_result = structured_user_command(
  "ReciteExtract",
  "extract",
  escaped(extract_output) .. " " .. escaped(valid))
local extract_artifact = extract_result.terminal.data.artifact
assert_true(type(extract_artifact) == "table"
  and extract_artifact.path.encoding == "utf8"
  and extract_artifact.path.value == extract_output
  and vim.fn.filereadable(extract_output) == 1,
  "ReciteExtract did not return its structured artifact")

local run_result = structured_user_command(
  "ReciteRun",
  "run",
  escaped(runtime_asset) .. " work " .. escaped(runtime_fixture))
local run_trace = run_result.terminal.data.trace
assert_true(type(run_trace) == "table"
  and run_trace.asset_id == runtime_asset
  and run_trace.block == "work"
  and type(run_trace.events) == "table"
  and #run_trace.events >= 2,
  "ReciteRun did not return the explicit structured runtime trace")
local work_line = run_trace.events[1]
local work_end = run_trace.events[#run_trace.events]
-- Source IDs retain the author label, while runtime trace line IDs use the
-- canonical anchor.  Keep the authored ID here and assert that the host's
-- structured projection preserves its exact anchor identity.
local expected_work_id = "work_001@2119548317bb586e3865"
local expected_work_anchor = expected_work_id:match("@(.+)$")
assert_true(work_line.type == "line"
  and expected_work_anchor ~= nil
  and work_line.line.id == expected_work_anchor
  and work_line.line.text == "Work waits.",
  "ReciteRun did not return the known work-block line")
assert_true(work_end.type == "end",
  "ReciteRun did not terminate with the structured end event")

local trace_result = structured_user_command(
  "ReciteTrace",
  "trace",
  escaped(runtime_asset) .. " work " .. escaped(runtime_fixture))
local trace = trace_result.terminal.data.trace
assert_true(vim.deep_equal(run_trace, trace),
  "ReciteRun and ReciteTrace did not preserve deterministic trace data")

local failure_before = #notifications
feed_command("ReciteRun")
wait_for(function()
  return #notifications > failure_before
end, "keyboard-reachable invalid command did not report failure")
local failure = notifications[#notifications]
assert_true(failure.level == vim.log.levels.ERROR,
  "command failure did not expose error severity")
assert_true(failure.message:find("inputs are incomplete or invalid", 1, true) ~= nil,
  "command failure was not textually observable: " .. failure.message)

local watch_before = #notifications
feed_command("ReciteWatchStart " .. escaped(project))
wait_for(function()
  return require("recite").watch_active() ~= nil
end, "keyboard-reachable watch start did not retain its child")
wait_for(function()
  return #notifications > watch_before and has_notification(notifications, "Recite watch:") ~= nil
end, "watch status was not textually observable")

feed_command("ReciteWatchStop")
wait_for(function()
  return require("recite").watch_active() == nil
end, "keyboard-reachable watch stop did not cleanly retire the child")
assert_true(has_notification(notifications, "Recite watch:") ~= nil,
  "watch stop did not leave a textual status record")

-- VimLeavePre is exercised by the shell process-group check after this clean
-- exit.  Keep the final command in the same host path used by an author.
vim.cmd("qa!")
