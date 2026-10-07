-- Structured command errors share protocol primitives, without editor lifecycle state.
local M = {}

function M.valid(value, protocol)
  local object = protocol.object
  local keys = protocol.keys
  local nonempty_string = protocol.nonempty_string
  local unique_strings = protocol.unique_strings
  local machine_path = protocol.machine_path
  local categories = {
    input = true,
    io = true,
    schema = true,
    compilation = true,
    asset = true,
    fixture = true,
    runtime = true,
    localisation = true,
    configuration = true,
    serialization = true,
    project = true,
    watch = true,
    benchmark = true,
    unsupported = true,
    internal = true,
  }
  local codes = {
    core_value = true,
    compile = true,
    compiled_value = true,
    decode_asset = true,
    diagnostics = true,
    diagnostic_rendering = true,
    dialogue_catalog_conflict = true,
    dialogue_catalog_plural_forms_conflict = true,
    dialogue_catalog_malformed = true,
    dialogue_catalog_missing_locale = true,
    dialogue_catalog_spec_invalid = true,
    dialogue_locale_invalid = true,
    diagnostic_code_malformed = true,
    diagnostic_code_unknown = true,
    fixture_choice_index_out_of_range = true,
    fixture_choice_not_in_prompt = true,
    ambiguous_fixture_choice = true,
    fixture_toml = true,
    asset_metadata = true,
    asset_not_file = true,
    io = true,
    malformed_compiled_asset = true,
    missing_path = true,
    invalid_project_root = true,
    missing_fixture_choice = true,
    no_inputs = true,
    output_overwrites_input = true,
    play_eof = true,
    play_invalid_input = true,
    play_interrupted = true,
    play_tui_requires_terminal = true,
    read = true,
    read_directory = true,
    runtime = true,
    preview = true,
    blocking_effect_needs_acknowledgement = true,
    bench = true,
    benchmark = true,
    bench_json = true,
    trace_json = true,
    schema_inspection = true,
    user_config = true,
    project_discovery = true,
    project_schema = true,
    ui_catalog = true,
    watch = true,
    watch_coordinator = true,
    watch_recovery = true,
    write = true,
    watch_preparation = true,
    watch_publisher = true,
  }
  local operations = {
    validate = true,
    compile = true,
    extract = true,
    run = true,
    trace = true,
    watch = true,
    load_asset = true,
    load_catalog = true,
    load_fixture = true,
    inspect_asset = true,
    collect_inputs = true,
    write_output = true,
    acknowledge_effect = true,
    resolve_path = true,
    discover_project = true,
    select_fixture_choice = true,
    start_watcher = true,
    watch_project = true,
    control = true,
    build = true,
    read = true,
    read_directory = true,
    write = true,
    load_schema = true,
    read_project_input = true,
    prepare_inputs = true,
    validate_project = true,
    prepare_request = true,
    prepare_targets = true,
    resolve_schema = true,
    prepare_publisher = true,
    resolve_project_root = true,
    validate_target = true,
  }
  if
    not object(value)
    or not categories[value.category]
    or not codes[value.code]
    or not operations[value.operation]
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
