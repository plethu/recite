-- Structured command errors share protocol primitives, without editor lifecycle state.
local M = {}
local vocabulary = require("recite_error_vocabulary")

function M.valid(value, protocol)
  local object = protocol.object
  local keys = protocol.keys
  local nonempty_string = protocol.nonempty_string
  local unique_strings = protocol.unique_strings
  local machine_path = protocol.machine_path
  if
    not object(value)
    or not vocabulary.errorCategories[value.category]
    or not vocabulary.errorCodes[value.code]
    or not vocabulary.operations[value.operation]
    or not keys(value, { "category", "code", "operation" }, { "path", "related_path", "details" })
    or value.path ~= nil and not machine_path(value.path)
    or value.related_path ~= nil and not machine_path(value.related_path)
  then
    return false
  end
  if value.details == nil then
    return true
  end
  local details = value.details
  if not object(details) or type(details.type) ~= "string" then
    return false
  end
  if details.type == "fixture_choice" then
    return keys(details, { "type", "choice", "prompt_keys" })
      and nonempty_string(details.choice)
      and unique_strings(details.prompt_keys)
  end
  if details.type == "fixture_choice_index" then
    return keys(details, { "type", "index", "choice_count", "prompt_keys" })
      and protocol.integer_in_range(details.index, "0", "18446744073709551615")
      and protocol.integer_in_range(details.choice_count, "0", "18446744073709551615")
      and unique_strings(details.prompt_keys)
  end
  if details.type == "ambiguous_fixture" then
    return keys(details, { "type", "block", "prompt_count" })
      and nonempty_string(details.block)
      and protocol.integer_in_range(details.prompt_count, "0", "18446744073709551615")
  end
  if details.type == "missing_fixture_choice" then
    return keys(details, { "type", "prompt_keys" }) and unique_strings(details.prompt_keys)
  end
  if details.type == "blocking_effect" then
    return keys(details, { "type", "effect" }) and nonempty_string(details.effect)
  end
  if details.type == "locale" then
    return keys(details, { "type", "field", "locale" })
      and nonempty_string(details.field)
      and nonempty_string(details.locale)
  end
  if details.type == "catalog_spec" then
    return keys(details, { "type", "spec" }) and nonempty_string(details.spec)
  end
  return (
    details.type == "watch"
    and keys(details, { "type", "kind" })
    and nonempty_string(details.kind)
  )
    or (
      details.type == "watch_target"
      and keys(details, { "type", "kind", "target" })
      and nonempty_string(details.kind)
      and nonempty_string(details.target)
    )
end

return M
