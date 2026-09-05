local protocol = require("recite.command_protocol")
local diagnostic = require("recite.diagnostic_protocol")

local M = {}

local function absolute(root, file)
  if file:sub(1, 1) == "/" then return vim.fn.fnamemodify(file, ":p") end
  return vim.fn.fnamemodify(root .. "/" .. file, ":p")
end

local function read_source(path)
  local handle, error = io.open(path, "rb")
  if not handle then return nil, error end
  local value = handle:read("*a")
  handle:close()
  return value
end

local function line_for(text, line)
  local start = 1
  local current = 1
  while current < line do
    local newline = text:find("\n", start, true)
    if not newline then return nil end
    start = newline + 1
    current = current + 1
  end
  local finish = text:find("\n", start, true)
  local value = text:sub(start, finish and finish - 1 or #text)
  if value:sub(-1) == "\r" then value = value:sub(1, -2) end
  return value
end

local function scalar_to_byte(line, column)
  local raw = protocol.integer_raw(column)
  if not raw or not protocol.integer_in_range(raw, "1", "18446744073709551615") then return nil end
  local target = tonumber(raw) - 1
  if not target or target < 0 then return nil end
  if target == 0 then return 0 end
  local index = 1
  local scalar = 0
  while index <= #line and scalar < target do
    local byte = line:byte(index)
    local width = 1
    if byte >= 0xF0 then width = 4
    elseif byte >= 0xE0 then width = 3
    elseif byte >= 0xC0 then width = 2 end
    if index + width - 1 > #line then return nil end
    index = index + width
    scalar = scalar + 1
  end
  if scalar ~= target then return nil end
  return index - 1
end

local function buffer_for(path)
  local bufnr = vim.fn.bufnr(path)
  if bufnr == -1 then
    bufnr = vim.fn.bufadd(path)
  end
  if not bufnr or bufnr == -1 then return nil end
  return bufnr
end

local function dirty(bufnr)
  return bufnr and vim.api.nvim_buf_is_valid(bufnr) and vim.bo[bufnr].modified
end

local function diagnostic_message(record)
  local rendered = diagnostic.render(record.presentation)
  return rendered or record.compatibility_message or record.code
end

local function to_diagnostic(record, project_root)
  if not diagnostic.valid(record) then error(protocol.error("invalid_diagnostic")) end
  local path = absolute(project_root, record.span.file)
  local root = vim.fn.fnamemodify(project_root, ":p"):gsub("/+$", "")
  if root == "" then root = "/" end
  local inside = root == "/" and path:sub(1, 1) == "/"
    or path == root or path:sub(1, #root + 1) == root .. "/"
  if not inside then
    error(protocol.error("diagnostic_path_outside_project"))
  end
  local source, read_error = read_source(path)
  if not source then error(protocol.error("diagnostic_source_unavailable", read_error)) end
  local start_line = protocol.integer_raw(record.span.start.line)
  local start_line_number = tonumber(start_line)
  local line = line_for(source, start_line_number)
  if not line then error(protocol.error("invalid_diagnostic_start")) end
  local start_col = scalar_to_byte(line, record.span.start.column)
  if not start_col then error(protocol.error("invalid_diagnostic_start")) end
  local end_col = start_col
  local end_line_number = start_line_number
  if record.span["end"] ~= nil and record.span["end"] ~= vim.NIL then
    end_line_number = tonumber(protocol.integer_raw(record.span["end"].line))
    local end_line = line_for(source, end_line_number)
    if not end_line then error(protocol.error("invalid_diagnostic_end")) end
    local end_start = scalar_to_byte(end_line, record.span["end"].column)
    if not end_start then error(protocol.error("invalid_diagnostic_end")) end
    -- CLI spans are inclusive. Neovim diagnostic ends are exclusive.
    local included = end_line:byte(end_start + 1)
    local width = included and (included >= 0xF0 and 4 or included >= 0xE0 and 3 or included >= 0xC0 and 2 or 1) or 0
    end_col = end_start + width
  end
  local bufnr = buffer_for(path)
  if not bufnr or dirty(bufnr) then return nil end
  local severity = ({ error = vim.diagnostic.severity.ERROR, warning = vim.diagnostic.severity.WARN,
    information = vim.diagnostic.severity.INFO, hint = vim.diagnostic.severity.HINT })[record.severity]
  return {
    bufnr = bufnr,
    diagnostic = {
      lnum = start_line_number - 1,
      col = start_col,
      end_lnum = end_line_number - 1,
      end_col = end_col,
      severity = severity,
      message = diagnostic_message(record),
      code = record.code,
      source = "recite-cli",
    },
  }
end

function M.new_namespace(name)
  return vim.api.nvim_create_namespace(name or "recite-cli")
end

function M.replace(namespace, records, project_root, known_buffers)
  if type(records) ~= "table" then error(protocol.error("invalid_diagnostics")) end
  local projected = {}
  for _, record in ipairs(records) do
    local value = to_diagnostic(record, project_root)
    if value then
      projected[value.bufnr] = projected[value.bufnr] or {}
      projected[value.bufnr][#projected[value.bufnr] + 1] = value.diagnostic
    end
  end
  vim.diagnostic.reset(namespace)
  for bufnr, diagnostics in pairs(projected) do
    vim.diagnostic.set(namespace, bufnr, diagnostics, { severity_sort = false })
    known_buffers[bufnr] = true
  end
  for bufnr in pairs(known_buffers) do
    if not projected[bufnr] then known_buffers[bufnr] = nil end
  end
end

function M.clear(namespace, known_buffers)
  vim.diagnostic.reset(namespace)
  for key in pairs(known_buffers) do known_buffers[key] = nil end
end

return M
