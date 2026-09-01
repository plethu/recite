local function fail(message)
  error("Neovim integration check: " .. message, 0)
end

local function assert_true(value, message)
  if not value then
    fail(message)
  end
end

local function wait_for(predicate, message)
  assert_true(vim.wait(10_000, predicate, 50), message)
end

local function escaped(path)
  return vim.fn.fnameescape(path)
end

local project = vim.env.RECITE_TEST_PROJECT
local valid = project .. "/core_language_spike.recite"
local invalid = project .. "/invalid.recite"

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
local target = { line = 6, character = 8 }
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
if rename ~= nil then
  assert_true(type(rename) == "table" and type(rename.documentChanges) == "table", "rename response was not a safe document change")
  assert_true(rename.documentChanges[1].textDocument.version == 1, "rename omitted the source version")
end
assert_true(vim.api.nvim_buf_get_lines(0, 0, -1, false)[7]:find("work", 1, true) ~= nil, "rename request mutated the buffer")

local actions = request(client, "textDocument/codeAction", {
  textDocument = { uri = uri },
  range = { start = target, ["end"] = { line = 6, character = 12 } },
  context = { diagnostics = {} },
})
assert_true(actions == nil or type(actions) == "table", "code action response was not safe")

vim.cmd("edit " .. escaped(invalid))
assert_true(vim.bo.filetype == "recite", "invalid .recite file lost its filetype")
wait_for(function()
  clients = vim.lsp.get_clients({ bufnr = 0, name = "recite-lsp" })
  return #clients > 0 and clients[1].initialized
end, "recite-lsp did not attach to the Recite buffer")
assert_true(clients[1].config.root_dir == project, "recite-lsp received the wrong project root")
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

local old_client_id = clients[1].id
local callback_seen = false
local capabilities = { workspace = { configuration = true } }
local init_options = { smoke = true }
recite.setup({
  lsp = {
    capabilities = capabilities,
    init_options = init_options,
    settings = { smoke = true },
    on_exit = function()
      callback_seen = true
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
recite.stop(client_id)
wait_for(function()
  return vim.lsp.get_client_by_id(client_id) == nil
end, "recite-lsp did not shut down cleanly")
-- The callback is delivered asynchronously by Neovim; observe it without
-- allowing an intentional stop to trigger crash recovery.
wait_for(function()
  return callback_seen
end, "caller on_exit was not preserved")
assert_true(#vim.lsp.get_clients({ bufnr = 0, name = "recite-lsp" }) == 0, "intentional shutdown restarted the client")

vim.cmd("qa!")
