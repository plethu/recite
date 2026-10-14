local command = require("recite.command_diagnostics")
local file = vim.fn.tempname() .. ".recite"
local root = vim.fn.fnamemodify(file, ":h")
local function write(text)
  local handle = assert(io.open(file, "wb"))
  handle:write(text)
  handle:close()
end
write("😀ab\rnext\r\nlast\n")
local namespace, known = command.new_namespace(), {}
local record = {
  version = 1,
  code = "RECITE_X001",
  severity = "error",
  span = { file = file, start = { line = 2, column = 1 }, ["end"] = { line = 2, column = 4 } },
  presentation = { id = "diagnostic-parse-001", arguments = vim.empty_dict() },
  related = {},
  help = vim.NIL,
  explanation = vim.NIL,
  compatibility_message = "test",
}
local records = {}
for _ = 1, 50 do
  records[#records + 1] = vim.deepcopy(record)
end
local original_open, reads = io.open, 0
io.open = function(path, mode)
  if path == file and mode == "rb" then
    reads = reads + 1
  end
  return original_open(path, mode)
end
command.replace(namespace, records, root, known)
local bufnr = vim.fn.bufnr(file)
local projected = vim.diagnostic.get(bufnr, { namespace = namespace })
assert(reads == 1 and #projected == 50)
assert(projected[1].lnum == 1 and projected[1].col == 0 and projected[1].end_col == 4)
write("😀ab\r更改\r\nlast\n")
record.span["end"].column = 2
command.replace(namespace, { record }, root, known)
projected = vim.diagnostic.get(bufnr, { namespace = namespace })
assert(reads == 2 and #projected == 1 and projected[1].end_col == 6)
local before = vim.diagnostic.get(bufnr, { namespace = namespace })
record.span.start.line = 99
local ok = pcall(command.replace, namespace, { record }, root, known)
assert(not ok and vim.deep_equal(before, vim.diagnostic.get(bufnr, { namespace = namespace })))
io.open = original_open
vim.api.nvim_buf_delete(bufnr, { force = true })
os.remove(file)
print("Diagnostic publication snapshots passed")
