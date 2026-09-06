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
feed_command("lua vim.diagnostic.goto_next()")
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
