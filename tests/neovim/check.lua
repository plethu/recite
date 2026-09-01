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
local old_client_id = clients[1].id
local callback_seen = false
local callback_errors = { on_exit = false, on_init = false }
-- Capture the integration's user-facing reports while keeping the headless
-- smoke output focused on failures.
vim.notify = function(message, level)
  if type(message) == "string" then
    if message:find("Recite on_exit callback failed", 1, true) then
      callback_errors.on_exit = true
      callback_errors.on_exit_level = level
    elseif message:find("Recite on_init callback failed", 1, true) then
      callback_errors.on_init = true
      callback_errors.on_init_level = level
    end
  end
end
local capabilities = { workspace = { configuration = true } }
local init_options = { smoke = true }
recite.setup({
  lsp = {
    capabilities = capabilities,
    init_options = init_options,
    settings = { smoke = true },
    on_exit = function()
      callback_seen = true
      error("hostile on_exit callback")
    end,
  },
})
local restarted
wait_for(function()
  restarted = vim.lsp.get_clients({ bufnr = 0, name = "recite-lsp" })
  return #restarted > 0 and restarted[1].initialized and restarted[1].id ~= old_client_id
end, "material setup change did not restart the Recite client")
client = restarted[1]
assert_true(client.config.settings.smoke == true, "settings were not preserved across restart")
assert_true(client.config.init_options.smoke == true, "init_options were not preserved across restart")
assert_true(client.config.capabilities.workspace.configuration == true, "capabilities were not preserved across restart")

local client_id = client.id
assert_true(vim.lsp.get_client_by_id(client_id) ~= nil, "recite-lsp disappeared before shutdown")

local buffer_a = vim.fn.bufnr(valid)
wait_for(function()
  local candidates = vim.lsp.get_clients({ bufnr = buffer_a, name = "recite-lsp" })
  return #candidates > 0 and candidates[1].initialized
end, "the first project buffer lost its client")
local client_a = vim.lsp.get_clients({ bufnr = buffer_a, name = "recite-lsp" })[1]
vim.cmd("edit " .. escaped(vim.env.RECITE_SECOND_PROJECT .. "/core_language_spike.recite"))
local buffer_b = vim.api.nvim_get_current_buf()
wait_for(function()
  clients = vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })
  return #clients > 0 and clients[1].initialized
end, "second project did not receive a separate Recite client")
local client_b = clients[1]
assert_true(client_b.config.root_dir == vim.env.RECITE_SECOND_PROJECT, "second project root was not isolated")
assert_true(client_b.id ~= client_a.id, "separate project roots shared a client")

-- A same-name/root client started by a caller remains external ownership.
local external_id = vim.lsp.start({
  name = "recite-lsp",
  cmd = { vim.env.RECITE_LSP },
  root_dir = vim.env.RECITE_SECOND_PROJECT,
}, { bufnr = buffer_b, reuse_client = function() return false end })
wait_for(function()
  local external = vim.lsp.get_client_by_id(external_id)
  return external ~= nil and external.initialized
end, "external same-root probe did not initialize")
local reused_id = recite.start(buffer_b)
assert_true(reused_id == client_b.id, "Recite did not reuse its owned same-root client")
assert_true(vim.lsp.get_client_by_id(external_id) ~= nil, "external same-root client was adopted or stopped")
assert_true(recite.stop(external_id) == false, "Recite claimed ownership of an external client")
vim.lsp.get_client_by_id(external_id):stop(true)

-- A configured root_dir must be used when recovering every open buffer.
recite.setup({
  lsp = {
    root_dir = function(bufnr)
      return vim.fn.fnamemodify(vim.api.nvim_buf_get_name(bufnr), ":p:h")
    end,
  },
})
wait_for(function()
  local a = vim.lsp.get_clients({ bufnr = buffer_a, name = "recite-lsp" })
  local b = vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })
  return #a > 0 and a[1].initialized and #b > 0 and b[1].initialized
end, "configured root_dir did not reattach both project buffers")
client_a = vim.lsp.get_clients({ bufnr = buffer_a, name = "recite-lsp" })[1]
client_b = vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })[1]
local crashed_a, crashed_b = client_a.id, client_b.id
assert_true(client_a.rpc and client_a.rpc.terminate, "Neovim RPC crash probe is unavailable")
assert_true(client_b.rpc and client_b.rpc.terminate, "Neovim RPC crash probe is unavailable")
client_a.rpc:terminate()
client_b.rpc:terminate()
local restarted_a, restarted_b
wait_for(function()
  restarted_a = vim.lsp.get_clients({ bufnr = buffer_a, name = "recite-lsp" })
  restarted_b = vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })
  return #restarted_a > 0 and restarted_a[1].initialized and restarted_a[1].id ~= crashed_a
    and #restarted_b > 0 and restarted_b[1].initialized and restarted_b[1].id ~= crashed_b
end, "crash recovery did not reattach every open project buffer")
wait_for(function()
  return callback_seen and callback_errors.on_exit
end, "throwing on_exit callback was not observed without blocking crash recovery")
assert_true(callback_errors.on_exit_level == vim.log.levels.ERROR,
  "throwing on_exit callback was not reported at error level")

-- Wait past the stability window, then verify the next crash uses the first
-- backoff interval rather than accumulating an old retry count.
vim.wait(1300, function() return false end, 50)
local stable_client = restarted_b[1]
local stable_id = stable_client.id
local clock = vim.uv or vim.loop
local crash_time = clock.hrtime()
stable_client.rpc:terminate()
local stable_restart
wait_for(function()
  local candidates = vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })
  stable_restart = candidates[1]
  return stable_restart ~= nil and stable_restart.initialized and stable_restart.id ~= stable_id
end, "stable client did not recover after a second crash")
assert_true((clock.hrtime() - crash_time) / 1e6 < 500, "stable recovery retained an excessive backoff")

-- A real server wrapper delays every server start by more than two seconds.
-- Each successful on_init must still reset the bounded restart
-- budget; otherwise the fourth post-init crash would be abandoned.
local delayed_init_calls = 0
recite.setup({
  lsp = {
    autostart = false,
    cmd = { vim.env.RECITE_DELAYED_LSP },
    root_dir = vim.env.RECITE_SECOND_PROJECT,
    on_init = function()
      delayed_init_calls = delayed_init_calls + 1
      error("hostile on_init callback")
    end,
  },
})
wait_for(function()
  return #vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" }) == 0
end, "previous owned clients did not stop before delayed wrapper probe")
local delayed_id = recite.start(buffer_b)
assert_true(delayed_id ~= nil, "delayed real-server wrapper did not start")
local delayed_client
wait_for(function()
  delayed_client = vim.lsp.get_client_by_id(delayed_id)
  return delayed_client ~= nil and delayed_client.initialized
end, "delayed real-server wrapper did not initialize")
wait_for(function()
  return delayed_init_calls == 1 and callback_errors.on_init
end, "throwing on_init callback was not observed after internal reset")
assert_true(callback_errors.on_init_level == vim.log.levels.ERROR,
  "throwing on_init callback was not reported at error level")

for attempt = 1, 4 do
  local crashed_id = delayed_id
  delayed_client = vim.lsp.get_client_by_id(crashed_id)
  assert_true(delayed_client ~= nil and delayed_client.initialized,
    "delayed client was not initialized before crash " .. attempt)
  assert_true(delayed_client.rpc and delayed_client.rpc.terminate,
    "Neovim RPC crash probe is unavailable for delayed client")
  delayed_client.rpc:terminate()
  wait_for(function()
    local candidates = vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })
    delayed_client = candidates[1]
    return delayed_client ~= nil and delayed_client.initialized and delayed_client.id ~= crashed_id
  end, "delayed post-init crash " .. attempt .. " did not recover within bounded policy")
  delayed_id = delayed_client.id
end
assert_true(delayed_init_calls == 5, "delayed wrapper did not deliver every successful on_init event")
assert_true(recite.stop(delayed_id), "delayed owned client did not accept intentional stop")
wait_for(function()
  return #vim.lsp.get_clients({ bufnr = buffer_a, name = "recite-lsp" }) == 0
    and #vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" }) == 0
end, "intentional shutdown did not stop all owned clients")
wait_for(function()
  return callback_seen
end, "caller on_exit was not preserved")

vim.cmd("qa!")
