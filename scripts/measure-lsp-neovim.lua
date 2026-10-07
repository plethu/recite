-- Run with: RECITE_PERF_ROOT=... RECITE_PERF_BINARY=... RECITE_PERF_OUTPUT=...
-- nvim --headless --clean -l scripts/measure-lsp-neovim.lua
-- Measures the installed adapter/client/diagnostic-store pipeline, not pixels.
local root = assert(vim.env.RECITE_PERF_ROOT)
local binary = assert(vim.env.RECITE_PERF_BINARY)
local output = assert(vim.env.RECITE_PERF_OUTPUT)
-- An explicit override is an experiment; absent it, use adapter defaults.
if vim.env.RECITE_PERF_DEBOUNCE then
  local start_client = vim.lsp.start
  vim.lsp.start = function(config, options)
    config.flags = vim.tbl_extend("force", config.flags or {}, {
      debounce_text_changes = assert(tonumber(vim.env.RECITE_PERF_DEBOUNCE)),
    })
    return start_client(config, options)
  end
end
vim.opt.runtimepath:prepend(vim.fn.getcwd() .. "/editors/recite-neovim")
local clock = vim.uv.hrtime
local published, started, elapsed
local handler = vim.lsp.handlers["textDocument/publishDiagnostics"]
vim.lsp.handlers["textDocument/publishDiagnostics"] = function(err, result, ctx, config)
  handler(err, result, ctx, config)
  if result and result.uri == vim.uri_from_bufnr(0) and result.version then
    published = result.version
    if started then
      elapsed = (clock() - started) / 1e6
    end
  end
end
local recite = require("recite")
recite.setup({ lsp = { cmd = { binary }, root_dir = root }, treesitter = { enabled = false } })
local source = vim.fn.glob(root .. "/src/*.recite", false, true)[1]
assert(source, "generated source missing")
vim.cmd.edit(vim.fn.fnameescape(source))
assert(
  vim.wait(30000, function()
    return published ~= nil
  end, 1),
  "initial diagnostics timeout"
)
local client = assert(vim.lsp.get_clients({ bufnr = 0, name = "recite-lsp" })[1])
local report = {
  host = vim.version(),
  headless = true,
  treesitter = false,
  debounce_text_changes_ms = client.flags.debounce_text_changes or 150,
  project = root,
  binary = binary,
  edit_to_diagnostic_store_ms = {},
  completion_ms = {},
}
local lines = vim.api.nvim_buf_get_lines(0, 0, -1, false)
local position
for index, line in ipairs(lines) do
  local col = line:find("-> block_", 1, true)
  if col then
    position = { line = index - 1, character = col + 2 }
    break
  end
end
assert(position, "completion probe missing")
for index = 1, 23 do
  local previous = published
  elapsed = nil
  started = clock()
  local keys =
    vim.api.nvim_replace_termcodes("Go# host edit " .. index .. "<Esc>", true, false, true)
  vim.api.nvim_feedkeys(keys, "xt", false)
  assert(
    vim.wait(10000, function()
      return published ~= previous
    end, 1),
    "edit diagnostics timeout"
  )
  if index > 2 then
    table.insert(report.edit_to_diagnostic_store_ms, elapsed)
  end
  started = nil
  assert(#vim.diagnostic.get(0) == 0, "unexpected diagnostics")
  local request_started = clock()
  local response = assert(client:request_sync("textDocument/completion", {
    textDocument = { uri = vim.uri_from_bufnr(0) },
    position = position,
  }, 10000, 0))
  assert(not response.err and response.result, "completion failed")
  if index > 2 then
    table.insert(report.completion_ms, (clock() - request_started) / 1e6)
  end
end
recite.stop(client.id)
assert(
  vim.wait(10000, function()
    return client:is_stopped()
  end, 1),
  "server did not stop"
)
vim.fn.writefile({ vim.json.encode(report) }, output)
vim.cmd("qa!")
