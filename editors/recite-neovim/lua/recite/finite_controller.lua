local protocol = require("recite.command_protocol")
local process = require("recite.command_process")
local diagnostics = require("recite.command_diagnostics")
local inputs = require("recite.command_inputs")

local M = {}

local function json(value)
  return vim.inspect(value)
end

local function invocation_id(prefix, generation)
  local clock = (vim.uv or vim.loop).hrtime()
  return string.format("nvim-%s-%d-%d", prefix, clock, generation)
end

-- Owns finite command scheduling and process lifetimes.  The command facade
-- remains responsible for argv grammar and watch setup; this boundary keeps
-- replacement, fencing, and disposal rules in one place.
function M.new(options)
  local state = assert(options.state)
  local report = assert(options.report)
  local watcher_active = options.watcher_active or function() return false end
  local controller = {}

  local function stop_process(session)
    if session and session.process then
      process.terminate_bounded(session.process, state.config.finite_stop_timeout_ms, state.config.finite_kill_timeout_ms, function()
        if session.closed or session.hung then return end
        session.hung = true
        state.finite_blocked = true
        report("neovim-command-protocol-failure", { detail = "finite_process_hung" }, vim.log.levels.ERROR)
      end)
    end
  end

  local function finite_failure(session, error)
    if session.error_settled or session.result_settled then return end
    session.error_settled = true
    if not error.silent then
      report(error.kind == "protocol" and "neovim-command-protocol-failure" or "neovim-command-failure",
        { detail = protocol.error_message(error) }, vim.log.levels.ERROR)
    end
    if session.on_error then
      local ok, callback_error = pcall(session.on_error, error)
      if not ok then
        report("neovim-callback-failed", { kind = "command", detail = tostring(callback_error) }, vim.log.levels.ERROR)
      end
    end
  end

  local function has_owned_finite()
    for _, session in pairs(state.finite_sessions) do
      if not session.owner_closed then return true end
    end
    return false
  end

  local launch_pending

  local function supersede(session)
    if session.owner_closed or session.error_settled or session.result_settled then
      session.retired = true
      return
    end
    session.retired = true
    finite_failure(session, { kind = "operational", code = "superseded", detail = "replaced by a newer invocation", silent = true })
  end

  launch_pending = function(session)
    local config, command, args = session.config, session.command, session.args
    local argv = { session.binary, command }
    vim.list_extend(argv, args)
    vim.list_extend(argv, { "--output-format", "structured", "--invocation-id", session.invocation_id })
    session.process = process.start_finite({
      argv = argv,
      cwd = session.cwd,
      max_bytes = config.max_finite_bytes,
      term_timeout_ms = config.finite_stop_timeout_ms,
      kill_timeout_ms = config.finite_kill_timeout_ms,
      system = config.system,
      command = command,
      invocation_id = session.invocation_id,
      on_spawn = function(child) session.child = child end,
      on_callback_failed = function(error)
        report("neovim-callback-failed", { kind = "command", detail = tostring(error) }, vim.log.levels.ERROR)
      end,
      on_error = function(error) finite_failure(session, error) end,
      on_hung = function()
        if session.closed or session.hung then return end
        session.hung = true
        state.finite_blocked = true
        report("neovim-command-protocol-failure", { detail = "finite_process_hung" }, vim.log.levels.ERROR)
        if state.finite_pending then
          local pending = state.finite_pending
          state.finite_pending = nil
          finite_failure(pending, { kind = "operational", code = "finite_process_hung", detail = "predecessor did not close", silent = true })
        end
      end,
      on_result = function(result)
        if session.retired or session.generation ~= state.finite_generation then return end
        if session.snapshot and not inputs.snapshot_current(session.snapshot) then
          finite_failure(session, { kind = "operational", code = "stale_snapshot", detail = "source changed during invocation" })
          return
        end
        if result.terminal.event == "command.error" then
          local error = vim.deepcopy(result.terminal.error)
          error.kind = "operational"
          error.detail = json(result.terminal.error)
          finite_failure(session, error)
          return
        end
        local data = result.terminal.data
        -- Watch owns its diagnostic namespace for its entire lifetime. A
        -- finite result may settle its caller, but cannot overwrite it.
        if watcher_active() then
          session.result_settled = true
          if session.on_result then session.on_result(result) end
          return
        end
        if data.diagnostics then
          local ok, error = pcall(diagnostics.replace, state.namespace, data.diagnostics,
            session.project_root or config.project_root or vim.fn.getcwd(), state.known_buffers)
          if not ok then finite_failure(session, error); return end
        end
        local message_id = result.terminal.status == "content_diagnostics"
          and "neovim-command-content-diagnostics" or "neovim-command-result"
        report(message_id, { command = command, detail = json(data) }, result.terminal.status == "content_diagnostics" and vim.log.levels.WARN or vim.log.levels.INFO)
        session.result_settled = true
        if session.on_result then session.on_result(result) end
      end,
      on_close = function()
        session.owner_closed = true
        session.retired = true
        state.finite_sessions[session.invocation_id] = nil
        if session.hung then
          state.finite_blocked = false
          for _, remaining in pairs(state.finite_sessions) do
            if remaining.hung and not remaining.owner_closed then state.finite_blocked = true; break end
          end
        end
        if state.finite_pending and not state.finite_blocked and not has_owned_finite() then
          local pending = state.finite_pending
          state.finite_pending = nil
          state.finite_sessions[pending.invocation_id] = pending
          launch_pending(pending)
        end
      end,
    })
  end

  function controller.execute(command, args, options_override)
    options_override = options_override or {}
    if state.finite_blocked then
      report("neovim-command-protocol-failure", { detail = "finite_process_hung" }, vim.log.levels.ERROR)
      return nil
    end
    local config = vim.tbl_deep_extend("force", vim.deepcopy(state.config), options_override.config or {})
    local binary, requested = inputs.command_binary(config)
    if not binary then
      report("neovim-command-cli-missing", { command = tostring(requested) }, vim.log.levels.ERROR)
      return nil
    end
    state.finite_generation = state.finite_generation + 1
    local generation = state.finite_generation
    local id = options_override.invocation_id or invocation_id(command, generation)
    if state.finite_pending then
      supersede(state.finite_pending)
      state.finite_pending = nil
    end
    local session = {
      generation = generation, invocation_id = id, command = command,
      args = vim.deepcopy(args), binary = binary, config = config,
      cwd = options_override.cwd or config.cwd,
      project_root = options_override.project_root,
      retired = false, owner_closed = false, process = nil,
      snapshot = options_override.snapshot, on_error = options_override.on_error,
      on_result = options_override.on_result,
    }
    state.finite_pending = session
    for _, previous in pairs(state.finite_sessions) do
      if not previous.owner_closed then
        supersede(previous)
        stop_process(previous)
      end
    end
    if not has_owned_finite() then
      state.finite_pending = nil
      state.finite_sessions[id] = session
      launch_pending(session)
    end
    return { invocation_id = id, generation = generation, session = session }
  end

  local function cancel(reason)
    state.finite_generation = state.finite_generation + 1
    if state.finite_pending then
      local pending = state.finite_pending
      state.finite_pending = nil
      finite_failure(pending, { kind = "operational", code = "cancelled", detail = reason, silent = true })
    end
    for _, session in pairs(state.finite_sessions) do
      finite_failure(session, { kind = "operational", code = "cancelled", detail = reason, silent = true })
      session.retired = true
      stop_process(session)
    end
  end

  function controller.dispose(reason)
    cancel(reason or "adapter disposed")
  end

  function controller.dispose_sync(reason)
    cancel(reason or "editor exit")
    local sessions = {}
    for _, session in pairs(state.finite_sessions) do sessions[#sessions + 1] = session end
    local function closed()
      for _, session in ipairs(sessions) do
        if not session.owner_closed then return false end
      end
      return true
    end
    vim.wait(state.config.finite_stop_timeout_ms or 250, closed, 10)
    if not closed() then
      for _, session in ipairs(sessions) do
        if not session.owner_closed then process.terminate(session.process, true) end
      end
      vim.wait(state.config.finite_kill_timeout_ms or 250, closed, 10)
    end
    if not closed() then
      report("neovim-command-protocol-failure", { detail = "finite_process_hung" }, vim.log.levels.ERROR)
    end
  end

  return controller
end

return M
