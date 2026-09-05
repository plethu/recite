local function fail(message)
  error("Neovim integration check: " .. message, 0)
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

local project = vim.env.RECITE_TEST_PROJECT
local valid = project .. "/core_language_spike.recite"
local invalid_project = vim.env.RECITE_INVALID_PROJECT
local invalid = invalid_project .. "/invalid.recite"

local function assert_isolated_neovim_paths()
  local expected_paths = {
    config = vim.env.XDG_CONFIG_HOME .. "/nvim",
    data = vim.env.XDG_DATA_HOME .. "/nvim",
    state = vim.env.XDG_STATE_HOME .. "/nvim",
    cache = vim.env.XDG_CACHE_HOME .. "/nvim",
  }
  for _, kind in ipairs({ "config", "data", "state", "cache" }) do
    local path = vim.fn.stdpath(kind)
    assert_true(path == expected_paths[kind],
      "Neovim " .. kind .. " path escaped the isolated XDG directory: " .. path)
  end

  local forbidden_user_roots = {
    vim.fn.expand("~/.config/nvim"),
    vim.fn.expand("~/.local/share/nvim"),
    vim.fn.expand("~/.local/state/nvim"),
    vim.fn.expand("~/.cache/nvim"),
    vim.fn.expand("~/.vim"),
  }
  local runtime_paths = vim.opt.runtimepath:get()
  assert_true(vim.tbl_contains(runtime_paths, vim.env.VIMRUNTIME),
    "Neovim system runtime was removed from runtimepath")
  for _, path in ipairs(runtime_paths) do
    for _, forbidden_root in ipairs(forbidden_user_roots) do
      local is_forbidden = path == forbidden_root
        or path:sub(1, #forbidden_root + 1) == forbidden_root .. "/"
      assert_true(not is_forbidden,
        "Neovim runtimepath included a user directory or local plugin: " .. path)
    end
  end
end

assert_isolated_neovim_paths()

vim.cmd("filetype on")
vim.cmd("edit " .. escaped(valid))
assert_true(vim.bo.filetype == "recite", ".recite file did not get the recite filetype")

local recite = require("recite")
local root = recite.root_dir(0)
assert_true(root == project, "root resolution did not select the nearest project manifest")

local function request(client, method, params, bufnr)
  local finished = false
  local result
  local request_error
  local accepted = client:request(method, params, function(err, response)
    request_error = err
    result = response
    finished = true
  end, bufnr or 0)
  assert_true(accepted, method .. " request was not accepted")
  wait_for(function()
    return finished
  end, method .. " request did not complete")
  assert_true(request_error == nil, method .. " request failed: " .. vim.inspect(request_error))
  return result
end

if vim.env.RECITE_PARSER_AVAILABLE == "1" then
  local started, start_error = pcall(vim.treesitter.start, 0, "recite")
  assert_true(started, "Tree-sitter failed to start: " .. tostring(start_error))
  local parser = vim.treesitter.get_parser(0, "recite")
  local tree = parser:parse()[1]
  assert_true(not tree:root():has_error(), "canonical fixture produced Tree-sitter recovery errors")
  local query = vim.treesitter.query.get("recite", "highlights")
  assert_true(query ~= nil, "Recite Tree-sitter highlight query was not found")
  local captures = {}
  for capture_id in query:iter_captures(tree:root(), 0) do
    captures[query.captures[capture_id]] = true
  end
  assert_true(captures.keyword == true, "Tree-sitter query did not produce a keyword capture")
  assert_true(captures["string.special"] == true, "Tree-sitter query did not produce prose captures")
end

local clients
wait_for(function()
  clients = vim.lsp.get_clients({ bufnr = 0, name = "recite-lsp" })
  return #clients > 0 and clients[1].initialized
end, "recite-lsp did not attach through the automatic runtimepath entry")
local client = clients[1]
assert_true(client.config.root_dir == project, "recite-lsp received the wrong project root")
assert_true(client.offset_encoding == "utf-16", "recite-lsp did not negotiate UTF-16 positions")
assert_true(client.config.cmd[1] == vim.env.RECITE_LSP, "pre-load options were not applied by the automatic plugin entry")

local uri = vim.uri_from_fname(valid)
local target = { line = 6, character = 7 }
local completion = request(client, "textDocument/completion", {
  textDocument = { uri = uri },
  position = { line = 6, character = 9 },
})
assert_true(type(completion) == "table", "completion response was not structured")
local completion_items = completion.items or completion
local saw_work = false
for _, item in ipairs(completion_items) do
  if item.label == "work" then
    saw_work = true
  end
end
assert_true(saw_work, "completion response omitted work")

local definition = request(client, "textDocument/definition", {
  textDocument = { uri = uri },
  position = target,
})
assert_true(type(definition) == "table" and definition.uri == uri, "definition did not preserve the source URI")
assert_true(definition.range.start.line == 13 and definition.range.start.character == 3, "definition range changed")

local hover = request(client, "textDocument/hover", {
  textDocument = { uri = uri },
  position = target,
})
assert_true(type(hover) == "table" and hover.contents ~= nil, "hover response was not structured")

local references = request(client, "textDocument/references", {
  textDocument = { uri = uri },
  position = target,
  context = { includeDeclaration = true },
})
assert_true(type(references) == "table" and #references == 2, "references response was not source ordered")
assert_true(references[1].uri == uri and references[1].range.start.line == 13
  and references[1].range.start.character == 3 and references[1].range["end"].line == 13
  and references[1].range["end"].character == 7, "references did not put the declaration first")
assert_true(references[2].uri == uri and references[2].range.start.line == 6
  and references[2].range.start.character == 7 and references[2].range["end"].line == 6
  and references[2].range["end"].character == 11, "references did not preserve source order")

local prepared = request(client, "textDocument/prepareRename", {
  textDocument = { uri = uri },
  position = target,
})
assert_true(prepared.placeholder == "work", "prepareRename returned the wrong placeholder")
local rename = request(client, "textDocument/rename", {
  textDocument = { uri = uri },
  position = target,
  newName = "renamed",
})
assert_true(type(rename) == "table" and type(rename.documentChanges) == "table", "rename response was not a safe document change: " .. vim.inspect(rename))
assert_true(#rename.documentChanges == 1, "rename returned an unexpected number of document changes")
assert_true(rename.documentChanges[1].textDocument.uri == uri, "rename changed the source URI")
assert_true(rename.documentChanges[1].textDocument.version ~= nil, "rename omitted the source version: " .. vim.inspect(rename))
local rename_edits = rename.documentChanges[1].edits
assert_true(#rename_edits == 2, "rename returned an unexpected number of edits")
assert_true(rename_edits[1].range.start.line == 6 and rename_edits[1].range.start.character == 7
  and rename_edits[1].range["end"].line == 6 and rename_edits[1].range["end"].character == 11
  and rename_edits[1].newText == "renamed", "rename reference edit was not exact")
assert_true(rename_edits[2].range.start.line == 13 and rename_edits[2].range.start.character == 3
  and rename_edits[2].range["end"].line == 13 and rename_edits[2].range["end"].character == 7
  and rename_edits[2].newText == "renamed", "rename declaration edit was not exact")
assert_true(vim.api.nvim_buf_get_lines(0, 0, -1, false)[7]:find("work", 1, true) ~= nil, "rename request mutated the buffer")

local missing = vim.env.RECITE_MISSING_PROJECT .. "/missing.recite"
vim.cmd("edit " .. escaped(missing))
wait_for(function()
  clients = vim.lsp.get_clients({ bufnr = 0, name = "recite-lsp" })
  return #clients > 0 and clients[1].initialized and #vim.diagnostic.get(0) > 0
end, "missing-ID fixture did not publish diagnostics")
client = clients[1]
local missing_uri = vim.uri_from_fname(missing)
local missing_diagnostics = vim.diagnostic.get(0)
local missing_id_diagnostic
for _, item in ipairs(missing_diagnostics) do
  if item.code == "RECITE_ID001" or item.user_data.lsp.code == "RECITE_ID001" then
    missing_id_diagnostic = item.user_data.lsp
  end
end
assert_true(missing_id_diagnostic ~= nil, "missing-ID diagnostic omitted RECITE_ID001")
local actions = request(client, "textDocument/codeAction", {
  textDocument = { uri = missing_uri },
  range = { start = { line = 2, character = 0 }, ["end"] = { line = 2, character = 1 } },
  context = { diagnostics = { missing_id_diagnostic }, only = { "quickfix" } },
})
assert_true(type(actions) == "table" and #actions > 0, "code action response omitted the missing-ID quickfix")
local action = actions[1]
local change = action.edit.documentChanges[1]
assert_true(action.kind == "quickfix", "missing-ID code action was not a quickfix")
assert_true(change.textDocument.uri == missing_uri and change.textDocument.version ~= nil, "code action edit did not preserve the source version: " .. vim.inspect(actions))
assert_true(#change.edits == 1, "missing-ID code action returned unexpected edits")
assert_true(change.edits[1].range.start.line == 2 and change.edits[1].range.start.character == 1,
  "missing-ID code action targeted the wrong insertion point")
assert_true(change.edits[1].newText == " line@34e5ee56e949afa2bbf3", "missing-ID code action invented the wrong stable ID: " .. vim.inspect(actions))

vim.cmd("edit " .. escaped(invalid))
assert_true(vim.bo.filetype == "recite", "invalid .recite file lost its filetype")
wait_for(function()
  clients = vim.lsp.get_clients({ bufnr = 0, name = "recite-lsp" })
  return #clients > 0 and clients[1].initialized
end, "recite-lsp did not attach to the Recite buffer")
assert_true(clients[1].config.root_dir == invalid_project, "recite-lsp received the wrong project root")
assert_true(clients[1].offset_encoding == "utf-16", "recite-lsp did not negotiate UTF-16 positions")

wait_for(function()
  return #vim.diagnostic.get(0) > 0
end, "recite-lsp did not publish diagnostics for malformed source")

local diagnostic = vim.diagnostic.get(0)[1]
-- vim.diagnostic normalises the protocol range to byte-oriented buffer
-- coordinates, while the server's wire range is UTF-16 (covered by the LSP
-- conformance tests).  These fields prove Neovim received a structured,
-- positioned diagnostic rather than a startup-only warning.
assert_true(type(diagnostic.lnum) == "number", "diagnostic did not contain a line position")
assert_true(type(diagnostic.col) == "number", "diagnostic did not contain a column position")
assert_true(type(diagnostic.message) == "string", "diagnostic did not contain a message")

vim.api.nvim_buf_set_lines(0, 0, -1, false, {
  ":: start default",
  ">",
  "  Hello.",
  "?",
  "  Stay.",
})
wait_for(function()
  return #vim.diagnostic.get(0) > 0
end, "missing-ID fixture did not publish diagnostics")
vim.cmd("edit " .. escaped(vim.env.RECITE_UNICODE_PROJECT .. "/unicode.recite"))
assert_true(vim.bo.fileformat == "dos", "CRLF fixture was not opened with DOS line endings")
wait_for(function()
  return #vim.diagnostic.get(0) > 0
end, "CRLF/non-BMP fixture did not publish diagnostics")
local unicode_diagnostic
for _, item in ipairs(vim.diagnostic.get(0)) do
  if item.lnum == 2 then
    unicode_diagnostic = item
  end
end
assert_true(unicode_diagnostic ~= nil, "unicode diagnostic was not positioned on the malformed line")
assert_true(unicode_diagnostic.col == 15, "Neovim did not project UTF-16 diagnostics to the expected byte column: " .. vim.inspect(unicode_diagnostic))
assert_true(unicode_diagnostic.user_data.lsp.range.start.character == 13, "the server did not retain the UTF-16 wire range")

clients = vim.lsp.get_clients({ bufnr = 0, name = "recite-lsp" })
assert_true(#clients > 0 and clients[1].initialized, "unicode buffer lost its initialized client")

local messages = require("recite_messages")
local health_module_path = vim.api.nvim_get_runtime_file("lua/recite/health.lua", true)
assert_true(#health_module_path == 1
  and health_module_path[1] == vim.env.RECITE_PLUGIN .. "/lua/recite/health.lua",
  "Neovim runtimepath did not expose the standard Recite health module")
assert_true(#vim.api.nvim_get_runtime_file("health/recite.lua", true) == 0,
  "Neovim runtimepath retained the non-discoverable Recite health module")
-- `:checkhealth` runs runtime discovery and must execute on the main event
-- loop, not nested inside the Lua chunk that drove the preceding LSP checks.
-- This also keeps Neovim 0.10 and 0.12 from reporting a spurious E5009
-- against their internally selected runtime path.
local health_finished = false
local health_error
vim.schedule(function()
  local ok, error = pcall(vim.cmd, "checkhealth recite")
  if ok then health_error = nil else health_error = error end
  health_finished = true
end)
wait_for(function() return health_finished end, "checkhealth did not complete")
assert_true(health_error == nil, "checkhealth raised a Lua/Vim error: " .. tostring(health_error))
local health_lines = vim.api.nvim_buf_get_lines(0, 0, -1, false)
local function health_report_contains(message)
  for _, line in ipairs(health_lines) do
    if line:find(message, 1, true) then
      return true
    end
  end
  return false
end

assert_true(package.loaded["recite.health"] ~= nil,
  ":checkhealth recite did not load the discoverable recite.health module")
assert_true(not health_report_contains('No healthcheck found for "recite" plugin.'),
  ":checkhealth recite reported that no healthcheck was found")
for _, message in ipairs({
  messages.format("neovim-health-filetype-ok"),
  messages.format("neovim-health-lsp-executable-found", { command = vim.env.RECITE_LSP }),
  messages.format("neovim-health-query-found"),
  messages.format("neovim-health-parser-found"),
  messages.format("neovim-health-open-buffer"),
}) do
  assert_true(health_report_contains(message),
    ":checkhealth recite omitted check result: " .. message)
end

vim.cmd("qa!")
