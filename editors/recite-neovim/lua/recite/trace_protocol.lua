-- Typed runtime trace validation. The CLI owns traversal and data shape;
-- Neovim only rejects malformed records before they reach a host projection.
local protocol = require("recite.command_protocol")
local M = {}

local I64_MIN, I64_MAX = "-9223372036854775808", "9223372036854775807"
local U32_MAX = "4294967295"
local U64_MAX, U128_MAX = "18446744073709551615", "340282366920938463463374607431768211455"
local reason_value

local function nullable(value, predicate)
  return value == vim.NIL or value == nil or predicate(value)
end

local function optional(value, predicate)
  return value == nil or predicate(value)
end

local function string_value(value) return type(value) == "string" end
local function string(value) return string_value(value) and value ~= "" end
local function positive(value) return protocol.integer_in_range(value, "1", U64_MAX) end
local function positive_or_zero(value) return protocol.integer_in_range(value, "0", U64_MAX) end

local function tagged(value, types, predicate)
  return protocol.object(value) and protocol.exact(value, { "type", "value" }) and types[value.type] and predicate(value.value, value.type)
end

local function scalar(value)
  return tagged(value, { identifier = true, string = true, integer = true, float = true, boolean = true }, function(inner, kind)
    if kind == "identifier" or kind == "string" then return string_value(inner) end
    if kind == "integer" then return protocol.integer_in_range(inner, I64_MIN, I64_MAX) end
    if kind == "float" then return type(inner) == "number" and inner == inner and inner ~= math.huge and inner ~= -math.huge end
    return type(inner) == "boolean"
  end)
end

local function trace_value(value)
  return tagged(value, { scalar = true, array = true }, function(inner, kind)
    if kind == "scalar" then return scalar(inner) end
    return protocol.array(inner) and protocol.every(inner, scalar)
  end)
end

local function source_span(value)
  return protocol.exact(value, { "file", "start_line", "start_column", "end_line", "end_column" })
    and string_value(value.file) and protocol.integer_in_range(value.start_line, "1", U32_MAX)
    and protocol.integer_in_range(value.start_column, "1", U32_MAX)
    and nullable(value.end_line, function(item) return protocol.integer_in_range(item, "1", U32_MAX) end)
    and nullable(value.end_column, function(item) return protocol.integer_in_range(item, "1", U32_MAX) end)
    and ((value.end_line == nil or value.end_line == vim.NIL) == (value.end_column == nil or value.end_column == vim.NIL))
end

local function metadata(value)
  return protocol.exact(value, { "key", "value" }) and string(value.key) and trace_value(value.value)
end

local function effect(value)
  if not protocol.exact(value, { "id", "mode", "function", "args", "source_span" }) or not string(value.id)
    or not vim.tbl_contains({ "deferred", "immediate", "blocking" }, value.mode) or not string(value["function"])
    or not protocol.array(value.args) or not source_span(value.source_span) then return false end
  return protocol.every(value.args, scalar)
end

local function plural_attempt(value)
  return protocol.keys(value, { "locale", "context", "key", "selected_arm", "outcome" }, { "selected_arm" })
    and string(value.locale) and string_value(value.context) and string_value(value.key)
    and nullable(value.selected_arm, function(arm) return protocol.integer_in_range(arm, "0", U64_MAX) end)
    and vim.tbl_contains({ "missing_plural_forms", "missing_entry", "missing_translation", "matched" }, value.outcome)
end

local function plural(value)
  return protocol.keys(value, { "singular_source_text", "plural_source_text", "count", "selected_arm", "attempts", "matched_locale", "matched_context", "matched_key", "matched_arm", "source_fallback_arm", "outcome" },
      { "matched_locale", "matched_context", "matched_key", "matched_arm", "source_fallback_arm" })
    and string_value(value.singular_source_text) and string_value(value.plural_source_text)
    and protocol.integer_in_range(value.count, I64_MIN, I64_MAX) and protocol.integer_in_range(value.selected_arm, "0", U64_MAX)
    and protocol.array(value.attempts) and protocol.every(value.attempts, plural_attempt)
    and nullable(value.matched_locale, string) and nullable(value.matched_context, string) and nullable(value.matched_key, string)
    and nullable(value.matched_arm, function(arm) return protocol.integer_in_range(arm, "0", U64_MAX) end)
    and nullable(value.source_fallback_arm, function(arm) return protocol.integer_in_range(arm, "0", U64_MAX) end)
    and vim.tbl_contains({ "translated", "english_source_fallback" }, value.outcome)
end

local function line(value)
  if not protocol.keys(value, { "id", "source_text", "text", "speaker", "metadata" }, { "plural" }) or not string(value.id)
    or not string_value(value.source_text) or not string_value(value.text) or not nullable(value.speaker, string)
    or not protocol.array(value.metadata) or not protocol.every(value.metadata, metadata) then return false end
  return optional(value.plural, plural)
end

local function prompt_identity(value)
  return protocol.exact(value, { "block", "line", "fixture_keys" }) and string(value.block)
    and nullable(value.line, string) and protocol.array(value.fixture_keys) and protocol.every(value.fixture_keys, string)
end

local function origin(value)
  if not protocol.object(value) or type(value.type) ~= "string" then return false end
  if value.type == "condition_call" then return protocol.exact(value, { "type", "function", "args" }) and string(value["function"]) and protocol.array(value.args) and protocol.every(value.args, reason_value) end
  return value.type == "requirement_expression" and protocol.exact(value, { "type", "source_text" }) and string_value(value.source_text)
end

reason_value = function(value)
  return tagged(value, { identifier = true, string = true, integer = true, float = true, boolean = true }, function(inner, kind)
    if kind == "identifier" or kind == "string" then return string_value(inner) end
    if kind == "integer" then return protocol.integer_in_range(inner, I64_MIN, I64_MAX) end
    if kind == "float" then return type(inner) == "number" and inner == inner and inner ~= math.huge and inner ~= -math.huge end
    return type(inner) == "boolean"
  end)
end

local function reason_arg(value)
  return protocol.exact(value, { "name", "value" }) and string(value.name) and reason_value(value.value)
end

local function availability_reason(value)
  return protocol.keys(value, { "id", "source_text", "localized_template", "text", "origin", "args" }, { "origin" })
    and string(value.id) and string_value(value.source_text) and string_value(value.localized_template) and string_value(value.text)
    and nullable(value.origin, origin) and protocol.array(value.args) and protocol.every(value.args, reason_arg)
end

local function reason_tree(value)
  if not protocol.object(value) or type(value.type) ~= "string" then return false end
  if value.type == "all" or value.type == "any" then return protocol.exact(value, { "type", "value" }) and protocol.array(value.value) and protocol.every(value.value, reason_tree) end
  if value.type == "reason" then return protocol.exact(value, { "type", "value" }) and availability_reason(value.value) end
  return value.type == "requirement_source_text" and protocol.exact(value, { "type", "value" }) and string_value(value.value)
end

local function availability(value)
  return protocol.keys(value, { "is_available", "primary_reason", "reason_tree" }, { "primary_reason", "reason_tree" })
    and type(value.is_available) == "boolean" and nullable(value.primary_reason, availability_reason)
    and nullable(value.reason_tree, reason_tree)
end

local function choice(value)
  return protocol.exact(value, { "id", "source_text", "text", "metadata", "is_available", "availability", "unavailable_reason" })
    and string(value.id) and string_value(value.source_text) and string_value(value.text) and protocol.array(value.metadata)
    and protocol.every(value.metadata, metadata) and type(value.is_available) == "boolean" and availability(value.availability)
    and nullable(value.unavailable_reason, string)
end

local function prompt(value)
  return protocol.exact(value, { "identity", "line", "choices" }) and prompt_identity(value.identity)
    and nullable(value.line, line) and protocol.array(value.choices) and protocol.every(value.choices, choice)
end

local function condition(value)
  return protocol.exact(value, { "query", "function", "arguments", "result" }) and string(value.query) and string(value["function"])
    and protocol.array(value.arguments) and protocol.every(value.arguments, scalar)
    and (type(value.result) == "boolean" or protocol.exact(value.result, { "enum" }) and string(value.result.enum))
end

local function event(value)
  if not protocol.object(value) or type(value.type) ~= "string" then return false end
  if value.type == "condition" then return protocol.exact(value, { "type", "condition" }) and condition(value.condition) end
  if value.type == "line" then return protocol.exact(value, { "type", "line" }) and line(value.line) end
  if value.type == "prompt" then return protocol.exact(value, { "type", "prompt" }) and prompt(value.prompt) end
  if value.type == "choice_selected" then return protocol.exact(value, { "type", "prompt", "choice" }) and prompt_identity(value.prompt) and string(value.choice) end
  if value.type == "effect" then return protocol.exact(value, { "type", "effect" }) and effect(value.effect) end
  if value.type == "acknowledgement" then return protocol.exact(value, { "type", "effect_id", "result" }) and string(value.effect_id) and value.result == "completed" end
  return value.type == "end" and protocol.exact(value, { "type", "deferred_effects" }) and protocol.array(value.deferred_effects) and protocol.every(value.deferred_effects, effect)
end

local function metrics(value)
  local counts = { "event_count", "line_count", "prompt_count", "choice_count", "condition_evaluation_count", "localization_lookup_count", "max_serialized_session_size_bytes" }
  if not protocol.exact(value, { "event_count", "line_count", "prompt_count", "choice_count", "condition_evaluation_count", "effect_count", "localization_lookup_count", "elapsed_traversal_time_ns", "max_serialized_session_size_bytes" })
    or not protocol.every(counts, function(key) return positive_or_zero(value[key]) end)
    or not protocol.integer_in_range(value.elapsed_traversal_time_ns, "0", U128_MAX)
    or not protocol.exact(value.effect_count, { "deferred", "immediate", "blocking" }) then return false end
  return protocol.every({ "deferred", "immediate", "blocking" }, function(key) return positive_or_zero(value.effect_count[key]) end)
end

function M.valid(value)
  if not protocol.keys(value, { "asset_id", "block", "events", "final_deferred_effects" }, { "dialogue_locale", "dialogue_locale_fallbacks", "metrics" })
    or not string(value.asset_id) or not string(value.block) or not optional(value.dialogue_locale, string)
    or not optional(value.dialogue_locale_fallbacks, function(items) return protocol.array(items) and protocol.every(items, string) end)
    or not protocol.array(value.events) or not protocol.every(value.events, event)
    or not protocol.array(value.final_deferred_effects) or not protocol.every(value.final_deferred_effects, effect) then return false end
  return optional(value.metrics, metrics)
end

M.valid_scalar = scalar
M.valid_effect = effect
M.valid_source_span = source_span

return M
