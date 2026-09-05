-- Finite command envelope and command-specific result shapes.  Keeping this
-- separate from the streaming watch validator makes terminal semantics easy
-- to audit and keeps the lossless lexer reusable by both protocols.
local protocol = require("recite.command_protocol")
local trace = require("recite.trace_protocol")
local diagnostic = require("recite.diagnostic_protocol")
local M = {}

local function valid_artifact(value)
  return protocol.exact(value, { "path", "size_bytes" }) and protocol.machine_path(value.path)
    and protocol.integer_in_range(value.size_bytes, "0", "18446744073709551615")
end

local function valid_finite(command, terminal, exit_code)
  if not protocol.keys(terminal, { "version", "sequence", "event", "command", "status", "exit_code", "data" }, { "invocation_id" })
    or terminal.event ~= "command.result" or (terminal.status ~= "success" and terminal.status ~= "content_diagnostics")
    or protocol.compare_integer(terminal.exit_code, exit_code) ~= 0
    or terminal.status == "success" and protocol.compare_integer(terminal.exit_code, 0) ~= 0
    or terminal.status == "content_diagnostics" and protocol.compare_integer(terminal.exit_code, 1) ~= 0 then return false end
  if not protocol.object(terminal.data) then return false end
  if command == "run" or command == "trace" then
    return terminal.status == "success" and protocol.exact(terminal.data, { "trace" }) and trace.valid(terminal.data.trace)
  end
  local valid_shape = protocol.exact(terminal.data, { "diagnostics" })
  if terminal.status == "success" and command == "compile" then valid_shape = protocol.exact(terminal.data, { "diagnostics", "artifact" }) end
  if terminal.status == "success" and command == "extract" then
    valid_shape = protocol.exact(terminal.data, { "diagnostics", "artifact" }) or protocol.exact(terminal.data, { "diagnostics", "entries" })
  end
  if not valid_shape or not protocol.array(terminal.data.diagnostics) then return false end
  for _, diagnostic_record in ipairs(terminal.data.diagnostics) do if not diagnostic.valid(diagnostic_record) then return false end end
  if terminal.status == "content_diagnostics" then return protocol.exact(terminal.data, { "diagnostics" }) end
  if command == "compile" then return valid_artifact(terminal.data.artifact) end
  if command == "extract" and terminal.data.artifact ~= nil then return valid_artifact(terminal.data.artifact) end
  if command == "extract" then
    return protocol.array(terminal.data.entries) and protocol.every(terminal.data.entries, function(entry)
      return protocol.exact(entry, { "context", "source_text", "plural_source_text", "comments", "reference" })
        and type(entry.context) == "string" and type(entry.source_text) == "string"
        and (entry.plural_source_text == vim.NIL or entry.plural_source_text == nil or type(entry.plural_source_text) == "string")
        and protocol.array(entry.comments) and protocol.every(entry.comments, function(comment) return type(comment) == "string" end)
        and (entry.reference == vim.NIL or entry.reference == nil or protocol.exact(entry.reference, { "file", "line", "column" })
          and type(entry.reference.file) == "string"
          and protocol.integer_in_range(entry.reference.line, "0", "4294967295")
          and protocol.integer_in_range(entry.reference.column, "0", "4294967295"))
    end)
  end
  return true
end

function M.parse(records, command, invocation_id, exit_code)
  if not vim.tbl_contains({ "validate", "compile", "extract", "run", "trace" }, command) then error(protocol.error("unsupported_command")) end
  if #records ~= 2 then error(protocol.error("finite_record_count")) end
  local started, terminal = records[1], records[2]
  if not protocol.keys(started, { "version", "sequence", "event", "command" }, { "invocation_id" }) or started.event ~= "command.started" then error(protocol.error("missing_started")) end
  protocol.validate_envelope(started, command, invocation_id, 0)
  if terminal.event == "command.result" then
    if not valid_finite(command, terminal, exit_code) then error(protocol.error("invalid_result")) end
  elseif terminal.event == "command.error" then
    if not protocol.keys(terminal, { "version", "sequence", "event", "command", "status", "exit_code", "error" }, { "invocation_id" })
      or terminal.status ~= "failure" or protocol.compare_integer(terminal.exit_code, 1) ~= 0 or exit_code ~= 1 or not protocol.valid_error(terminal.error) then error(protocol.error("invalid_error")) end
  else error(protocol.error("missing_terminal")) end
  protocol.validate_envelope(terminal, command, invocation_id, 1)
  return { records = records, terminal = terminal }
end

return M
