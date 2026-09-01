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
local valid = project .. "/language_pressure.recite"
local invalid = project .. "/invalid.recite"

vim.cmd("filetype on")
vim.cmd("edit " .. escaped(valid))
assert_true(vim.bo.filetype == "recite", ".recite file did not get the recite filetype")

local recite = require("recite")
local root = recite.root_dir(0)
assert_true(root == project, "root resolution did not select the nearest project manifest")

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

vim.cmd("edit " .. escaped(invalid))
assert_true(vim.bo.filetype == "recite", "invalid .recite file lost its filetype")
local clients
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

local client_id = clients[1].id
local client = vim.lsp.get_client_by_id(client_id)
assert_true(client ~= nil, "recite-lsp disappeared before shutdown")
client:stop(true)
wait_for(function()
  return vim.lsp.get_client_by_id(client_id) == nil
end, "recite-lsp did not shut down cleanly")

vim.cmd("qa!")
