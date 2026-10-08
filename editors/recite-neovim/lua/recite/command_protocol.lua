-- The CLI command protocol is deliberately kept separate from the LSP
-- lifecycle.  This module only accepts the versioned wire contract; it does
-- not know about buffers, diagnostics, or editor UI.
local M = {}
local lossless = require("recite.command_json")

M.PROTOCOL_VERSION = 1
M.MAX_RECORD_BYTES = 4 * 1024 * 1024
M.MAX_FINITE_BYTES = 32 * 1024 * 1024
M.MAX_STDERR_BYTES = 4 * 1024 * 1024

local function is_list(value)
  if type(value) ~= "table" then
    return false
  end
  if vim.islist then
    return vim.islist(value)
  end
  if vim.type_idx and value[vim.type_idx] == vim.types.array then
    return true
  end
  local count = 0
  for key in pairs(value) do
    if type(key) ~= "number" or key < 1 or key ~= math.floor(key) then
      return false
    end
    count = count + 1
  end
  if count == 0 then
    return false
  end
  for index = 1, count do
    if value[index] == nil then
      return false
    end
  end
  return true
end

local function object(value)
  return type(value) == "table" and not is_list(value)
end

local function array(value)
  return type(value) == "table" and is_list(value)
end

local function keys(value, required, optional)
  if not object(value) then
    return false
  end
  local allowed = {}
  for _, key in ipairs(required or {}) do
    if value[key] == nil then
      return false
    end
    allowed[key] = true
  end
  for _, key in ipairs(optional or {}) do
    allowed[key] = true
  end
  for key in pairs(value) do
    if not allowed[key] then
      return false
    end
  end
  return true
end

local function string_value(value)
  return type(value) == "string"
end

local function nonempty_string(value)
  return string_value(value) and value ~= ""
end

local function positive(value)
  local raw = M.integer_raw(value)
  return raw ~= nil and M.compare_integer(raw, "1") >= 0
end

local function exact(value, expected)
  return keys(value, expected, {}) and #expected == vim.tbl_count(value)
end

local function protocol_error(code, detail)
  return { kind = "protocol", code = code, detail = detail }
end

function M.error(code, detail)
  return protocol_error(code, detail)
end

function M.error_message(error)
  if type(error) ~= "table" then
    return tostring(error)
  end
  if error.detail and error.detail ~= "" then
    return string.format("%s: %s", error.code or "protocol_failure", error.detail)
  end
  return error.code or error.kind or "protocol_failure"
end

M.integer = lossless.integer
M.is_integer = lossless.is_integer
M.integer_raw = lossless.integer_raw
M.compare_integer = lossless.compare
M.integer_in_range = lossless.in_range
M.increment_integer = lossless.increment
M.parse_json = lossless.parse

local Parser = {}
Parser.__index = Parser

function M.new_parser(max_record_bytes, max_total_bytes)
  return setmetatable({
    max_record_bytes = max_record_bytes or M.MAX_RECORD_BYTES,
    max_total_bytes = max_total_bytes,
    buffer = "",
    total_bytes = 0,
    finished = false,
  }, Parser)
end

function Parser:push(chunk)
  if self.finished then
    error(protocol_error("records_after_end"))
  end
  if type(chunk) ~= "string" then
    error(protocol_error("invalid_chunk"))
  end
  self.total_bytes = self.total_bytes + #chunk
  if self.max_total_bytes and self.total_bytes > self.max_total_bytes then
    error(protocol_error("stream_too_large"))
  end
  self.buffer = self.buffer .. chunk
  local records = {}
  while true do
    local newline = self.buffer:find("\n", 1, true)
    if not newline then
      break
    end
    local line = self.buffer:sub(1, newline - 1)
    self.buffer = self.buffer:sub(newline + 1)
    if line:sub(-1) == "\r" then
      error(protocol_error("carriage_return_record"))
    end
    if line == "" then
      error(protocol_error("empty_record"))
    end
    if #line > self.max_record_bytes then
      error(protocol_error("record_too_large"))
    end
    local ok, value = pcall(M.parse_json, line)
    if not ok then
      error(protocol_error("invalid_json", tostring(value)))
    end
    if not object(value) then
      error(protocol_error("record_not_object"))
    end
    records[#records + 1] = value
  end
  -- The limit applies to the currently incomplete record, not to the
  -- lifetime of a stream. A chunk may contain many complete records.
  if #self.buffer > self.max_record_bytes then
    error(protocol_error("record_too_large"))
  end
  return records
end

function Parser:finish()
  if self.finished then
    return
  end
  self.finished = true
  if self.buffer ~= "" then
    error(protocol_error("truncated_record"))
  end
end

function M.validate_envelope(record, command, invocation_id, sequence)
  if
    record.version ~= M.PROTOCOL_VERSION
    or record.command ~= command
    or M.compare_integer(record.sequence, sequence) ~= 0
    or type(record.event) ~= "string"
  then
    error(protocol_error("invalid_envelope"))
  end
  if invocation_id ~= nil and record.invocation_id ~= invocation_id then
    error(protocol_error("invocation_mismatch"))
  end
end

local function machine_path(value)
  if not exact(value, { "encoding", "value" }) then
    return false
  end
  if value.encoding == "utf8" then
    return string_value(value.value)
  end
  if value.encoding == "unix_bytes" then
    return string_value(value.value)
      and value.value:match("^[0-9a-f]*$") ~= nil
      and #value.value % 2 == 0
  end
  if value.encoding == "windows_wtf16" then
    return array(value.value)
      and M.every(value.value, function(unit)
        return M.integer_in_range(unit, "0", "65535")
      end)
  end
  return false
end

local function machine_path_value(value)
  if not machine_path(value) then
    return nil
  end
  if value.encoding == "utf8" then
    return value.value
  end
  if value.encoding ~= "unix_bytes" or vim.fn.has("win32") == 1 then
    return nil
  end
  local bytes = {}
  for index = 1, #value.value, 2 do
    local byte = tonumber(value.value:sub(index, index + 1), 16)
    if not byte or byte == 0 then
      return nil
    end
    bytes[#bytes + 1] = string.char(byte)
  end
  return table.concat(bytes)
end

function M.unique_strings(value)
  if not array(value) then
    return false
  end
  local seen = {}
  for _, item in ipairs(value) do
    if not string_value(item) or seen[item] then
      return false
    end
    seen[item] = true
  end
  return true
end

M.machine_path = machine_path
M.machine_path_value = machine_path_value
M.object = object
M.array = array
M.exact = exact
M.keys = keys
function M.every(values, predicate)
  if not array(values) then
    return false
  end
  for _, value in ipairs(values) do
    if not predicate(value) then
      return false
    end
  end
  return true
end
M.nonempty_string = nonempty_string
M.positive = positive

local errors = require("recite.error_protocol")
function M.valid_error(value)
  return errors.valid(value, M)
end

return M
