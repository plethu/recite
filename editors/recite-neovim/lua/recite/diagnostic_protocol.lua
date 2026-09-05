-- Locale-neutral CLI diagnostic validation. Presentation remains a typed
-- Recite UI projection; this module only checks the shared wire shape.
local protocol = require("recite.command_protocol")
local diagnostic_registry = require("recite_diagnostics")
local M = {}

local function string_value(value) return type(value) == "string" end
local function nullable(value, predicate) return value == nil or value == vim.NIL or predicate(value) end
local U32_MAX = "4294967295"

local function source_position(value)
  return protocol.exact(value, { "line", "column" })
    and protocol.integer_in_range(value.line, "1", U32_MAX)
    and protocol.integer_in_range(value.column, "1", U32_MAX)
end

function M.valid_argument(value)
  if not protocol.exact(value, { "type", "value" }) or not vim.tbl_contains({ "string", "integer", "float", "boolean" }, value.type) then return false end
  if value.type == "string" then return string_value(value.value) end
  if value.type == "integer" then return protocol.integer_in_range(value.value, "-9223372036854775808", "9223372036854775807") end
  if value.type == "float" then return type(value.value) == "number" and value.value == value.value and value.value ~= math.huge and value.value ~= -math.huge end
  return type(value.value) == "boolean"
end

function M.valid_span(value)
  return protocol.exact(value, { "file", "start", "end" }) and string_value(value.file) and value.file ~= ""
    and source_position(value.start)
    and (value["end"] == nil or value["end"] == vim.NIL or source_position(value["end"])
      and (protocol.compare_integer(value["end"].line, value.start.line) > 0 or protocol.compare_integer(value["end"].line, value.start.line) == 0
        and protocol.compare_integer(value["end"].column, value.start.column) >= 0))
end

M.valid_source_position = source_position

function M.valid_presentation(value)
  if not protocol.keys(value, { "id", "arguments" }) or not string_value(value.id) or not value.id:match("^[a-z][a-z0-9%-]*$") or not protocol.object(value.arguments) then return false end
  local definition = diagnostic_registry[value.id]
  if not definition then return false end
  for name, argument in pairs(value.arguments) do
    if not name:match("^[a-z][a-z0-9_]*$") or not M.valid_argument(argument) then return false end
  end
  local expected = {}
  for _, argument in ipairs(definition.arguments) do expected[argument.name] = argument.type end
  if vim.tbl_count(value.arguments) ~= #definition.arguments then return false end
  for name, argument in pairs(value.arguments) do
    if expected[name] ~= argument.type then return false end
  end
  return true
end

function M.render(value)
  if not M.valid_presentation(value) then return nil end
  local definition = diagnostic_registry[value.id]
  if not definition or not definition.template then return nil end
  return (definition.template:gsub("{%$([%w_]+)}", function(name)
    local argument = value.arguments[name]
    if not argument then return "" end
    return protocol.integer_raw(argument.value) or tostring(argument.value)
  end))
end

function M.valid_explanation(value)
  if not protocol.exact(value, { "meaning", "common_causes", "remediation" }) or not M.valid_presentation(value.meaning)
    or not protocol.array(value.common_causes) or not protocol.array(value.remediation) then return false end
  return protocol.every(value.common_causes, M.valid_presentation) and protocol.every(value.remediation, M.valid_presentation)
end

function M.valid(record)
  if not protocol.keys(record, { "version", "code", "severity", "span", "presentation", "related", "help", "explanation", "compatibility_message" })
    or record.version ~= 1 or not string_value(record.code) or not record.code:match("^[A-Z][A-Z0-9]*_[A-Z0-9][A-Z0-9]*$")
    or not vim.tbl_contains({ "error", "warning", "information", "hint" }, record.severity)
    or not M.valid_span(record.span) or not M.valid_presentation(record.presentation) or not protocol.array(record.related) then return false end
  for _, related in ipairs(record.related) do
    if not protocol.exact(related, { "span", "presentation" }) or not M.valid_span(related.span) or not M.valid_presentation(related.presentation) then return false end
  end
  if not nullable(record.help, M.valid_presentation) or not nullable(record.explanation, M.valid_explanation) then return false end
  return nullable(record.compatibility_message, string_value)
end

return M
