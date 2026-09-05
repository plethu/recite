local protocol = require("recite.command_protocol")
local diagnostics = require("recite.command_diagnostics")
local watch = require("recite.watch")
local messages = require("recite_messages")
local inputs = require("recite.command_inputs")
local finite_controller = require("recite.finite_controller")

local M = {}

local function user_notify(id, arguments, level)
  local ok, text = pcall(messages.format, id, arguments or {})
  vim.notify(ok and text or id, level)
end

function M.new(options)
  local state = {
    config = vim.tbl_deep_extend("force", {
      binary = "recite",
      cwd = nil,
      project_root = nil,
      max_finite_bytes = protocol.MAX_FINITE_BYTES,
      max_watch_bytes = protocol.MAX_RECORD_BYTES,
      finite_stop_timeout_ms = 250,
      finite_kill_timeout_ms = 250,
      watch_cancel_grace_ms = 100,
      watch_stop_timeout_ms = 1500,
      watch_force_kill_delay_ms = 100,
      watch_teardown_timeout_ms = 500,
    }, options.config or {}),
    root_dir = options.root_dir,
    namespace = diagnostics.new_namespace(),
    watch_namespace = diagnostics.new_namespace("recite-cli-watch"),
    known_buffers = {},
    watch_buffers = {},
    finite_generation = 0,
    finite_sessions = {},
    finite_pending = nil,
    finite_blocked = false,
  }
  local adapter = {}

  local function report(id, arguments, level)
    user_notify(id, arguments, level)
  end

  local function clear_diagnostics()
    diagnostics.clear(state.namespace, state.known_buffers)
  end

  local watcher = watch.new({
    config = state.config,
    notify = report,
    on_started = function() clear_diagnostics() end,
    clear_diagnostics = function() diagnostics.clear(state.watch_namespace, state.watch_buffers) end,
    replace_diagnostics = function(records, root)
      diagnostics.replace(state.watch_namespace, records, root, state.watch_buffers)
    end,
  })

  local finite = finite_controller.new({
    state = state,
    report = report,
    watcher_active = function() return watcher.active() ~= nil end,
  })

  local function prepare_source(options)
    local root = inputs.project_root(options, state.config.project_root or state.root_dir)
    local paths = inputs.paths_for(options, root)
    if not paths then report("neovim-command-input-invalid", {}, vim.log.levels.ERROR); return nil end
    local snapshot, error = inputs.clean_snapshot(root)
    if error then report("neovim-command-document-unsaved", {}, vim.log.levels.ERROR); return nil end
    return root, paths, snapshot
  end

  local function ensure_derived_output_parent(output, config)
    if state.finite_blocked then
      report("neovim-command-protocol-failure", { detail = "finite_process_hung" }, vim.log.levels.ERROR)
      return false
    end
    local binary, requested = inputs.command_binary(config)
    if not binary then
      report("neovim-command-cli-missing", { command = tostring(requested) }, vim.log.levels.ERROR)
      return false
    end
    local parent = vim.fn.fnamemodify(output, ":h")
    if vim.fn.isdirectory(parent) == 1 then return true end
    local ok, created = pcall(vim.fn.mkdir, parent, "p")
    if ok and created == 1 and vim.fn.isdirectory(parent) == 1 then return true end
    report("neovim-command-failure", { detail = "unable to create derived compile output directory: " .. parent }, vim.log.levels.ERROR)
    return false
  end

  function adapter.validate(options_override)
    local root, paths, snapshot = prepare_source(options_override or {})
    if not root then return nil end
    return finite.execute("validate", paths, { project_root = root, cwd = root, snapshot = snapshot, config = options_override and options_override.config, invocation_id = options_override and options_override.invocation_id, on_result = options_override and options_override.on_result, on_error = options_override and options_override.on_error })
  end

  function adapter.compile(options_override)
    options_override = options_override or {}
    local root, paths, snapshot = prepare_source(options_override)
    if not root then return nil end
    local output = options_override.output or state.config.compile_output or root .. "/build/dialogue.recitec"
    output = inputs.absolute(output)
    for _, path in ipairs(paths) do
      if inputs.absolute(path) == output then report("neovim-command-input-invalid", {}, vim.log.levels.ERROR); return nil end
    end
    if not options_override.output then
      local config = vim.tbl_deep_extend("force", vim.deepcopy(state.config), options_override.config or {})
      if not ensure_derived_output_parent(output, config) then return nil end
    end
    if not options_override.output then report("neovim-command-output-derived", { path = output }, vim.log.levels.INFO) end
    local args = { "--output", output }
    vim.list_extend(args, paths)
    return finite.execute("compile", args, { project_root = root, cwd = options_override.cwd or root, snapshot = snapshot, config = options_override.config, invocation_id = options_override.invocation_id, on_result = options_override.on_result, on_error = options_override.on_error })
  end

  function adapter.extract(options_override)
    options_override = options_override or {}
    local root, paths, snapshot = prepare_source(options_override)
    if not root then return nil end
    local args = {}
    if options_override.output then args = { "--output", inputs.absolute(options_override.output) } end
    vim.list_extend(args, paths)
    return finite.execute("extract", args, { project_root = root, cwd = options_override.cwd or root, snapshot = snapshot, config = options_override.config, invocation_id = options_override.invocation_id, on_result = options_override.on_result, on_error = options_override.on_error })
  end

  local function runtime(options_override, command)
    options_override = options_override or {}
    local asset = options_override.asset
    local fixture = options_override.fixture
    local block = options_override.block
    if type(asset) ~= "string" or asset == "" or vim.fn.filereadable(asset) ~= 1
      or type(fixture) ~= "string" or fixture == "" or vim.fn.filereadable(fixture) ~= 1
      or type(block) ~= "string" or block == "" then
      report("neovim-command-input-invalid", {}, vim.log.levels.ERROR)
      return nil
    end
    asset, fixture = inputs.absolute(asset), inputs.absolute(fixture)
    return finite.execute(command, { asset, "--block", block, "--fixture", fixture }, { cwd = options_override.cwd or vim.fn.fnamemodify(asset, ":h"), config = options_override.config, invocation_id = options_override.invocation_id, on_result = options_override.on_result, on_error = options_override.on_error })
  end

  function adapter.run(options_override) return runtime(options_override, "run") end
  function adapter.trace(options_override) return runtime(options_override, "trace") end

  function adapter.watch_start(options_override)
    local config = vim.tbl_deep_extend("force", vim.deepcopy(state.config), options_override or {})
    config.project_root = inputs.project_root(options_override or {}, state.config.project_root or state.root_dir)
    watcher.configure(config)
    return watcher.start(config)
  end
  function adapter.watch_stop() return watcher.stop() end
  function adapter.watch_active() return watcher.active() end
  function adapter.clear_diagnostics() clear_diagnostics() end
  function adapter.configure(config)
    local next_config = vim.tbl_deep_extend("force", vim.deepcopy(state.config), config or {})
    if vim.deep_equal(next_config, state.config) then return false end
    finite.cancel("configuration changed")
    watcher.reconfigure()
    clear_diagnostics()
    state.config = next_config
    watcher.configure(state.config)
    return true
  end
  function adapter.dispose()
    watcher.dispose()
    finite.dispose("adapter disposed")
    clear_diagnostics()
    diagnostics.clear(state.watch_namespace, state.watch_buffers)
  end

  function adapter.dispose_sync()
    watcher.dispose_sync()
    finite.dispose_sync("editor exit")
    clear_diagnostics()
    diagnostics.clear(state.watch_namespace, state.watch_buffers)
  end

  return adapter
end

return M
