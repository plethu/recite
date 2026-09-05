local protocol = require("recite.command_protocol")
local process = require("recite.command_process")
local timer = require("recite.timer")
local watch_protocol = require("recite.watch_protocol")

local M = {}

local function json(value)
  return vim.inspect(value)
end

local function absolute(path)
  local value = vim.fn.fnamemodify(path, ":p")
  return vim.fs and vim.fs.normalize and vim.fs.normalize(value) or value
end

local function timing(configuration)
  return {
    cancel_grace_ms = configuration.watch_cancel_grace_ms or 100,
    stop_timeout_ms = configuration.watch_stop_timeout_ms or 1500,
    force_kill_delay_ms = configuration.watch_force_kill_delay_ms or 100,
    teardown_timeout_ms = configuration.watch_teardown_timeout_ms or 500,
  }
end

function M.new(options)
  local state = {
    config = options.config,
    notify = options.notify,
    on_started = options.on_started,
    replace_diagnostics = options.replace_diagnostics,
    clear_diagnostics = options.clear_diagnostics,
    active = nil,
    generation = 0,
  }

  local adapter = {}

  local function report(id, arguments, level)
    state.notify(id, arguments, level)
  end

  local function clear_timer(session, field)
    if session[field] then
      timer.stop(session[field])
      session[field] = nil
    end
  end

  local function retire(session)
    if session.retired then return end
    session.retired = true
    clear_timer(session, "stop_timer")
    clear_timer(session, "force_timer")
    clear_timer(session, "recovery_timer")
    clear_timer(session, "fatal_timer")
    clear_timer(session, "hung_timer")
    if state.active == session then state.active = nil end
    state.clear_diagnostics()
  end

  local function force_recovery(session)
    if session.retired or session.closed then return end
    process.terminate(session.transport, true)
    if session.hung_timer then return end
    session.hung_timer = timer.after(session.timing.teardown_timeout_ms, function()
      session.hung_timer = nil
      if session.retired or session.closed then return end
      session.hung = true
      -- A process which ignored both TERM and KILL is retained as a tombstone.
      -- It owns the watch slot until the actual on_exit callback arrives.
      report("neovim-command-protocol-failure", { detail = "watch_process_hung" }, vim.log.levels.ERROR)
    end)
  end

  local function recover(session)
    if session.retired or session.recovery then return end
    session.recovery = true
    session.recovery_timer = timer.after(session.timing.cancel_grace_ms, function()
      session.recovery_timer = nil
      if session.retired or session.closed then return end
      process.terminate(session.transport, false)
      session.force_timer = timer.after(session.timing.force_kill_delay_ms, function()
        session.force_timer = nil
        if session.retired or session.closed then return end
        process.terminate(session.transport, true)
        force_recovery(session)
      end)
    end)
  end

  local function failure(session, error)
    if state.active ~= session or session.retired or session.failed then return end
    session.failed = true
    if not session.fenced then
      report("neovim-command-protocol-failure", { detail = protocol.error_message(error) }, vim.log.levels.ERROR)
    end
    recover(session)
  end

  local function request_cancel(session)
    if session.stop_requested then return true end
    session.stop_requested = true
    local control = vim.json.encode({ version = 1, command = "watch", action = "cancel", invocation_id = session.invocation_id }) .. "\n"
    if not process.write(session.transport, control) then
      failure(session, protocol.error("cancel_write_failed"))
      return false
    end
    return true
  end

  local function record(session, record_value)
    if state.active ~= session or session.retired or session.fenced or session.recovery then return end
    local ok, error = pcall(session.validator.consume, session.validator, record_value)
    if not ok then failure(session, error); return end
    if record_value.event == "watch.started" then
      session.project_root = session.validator.project_root
      if state.on_started then state.on_started(session.project_root) end
      report("neovim-command-watch-status", { detail = json(record_value.data) }, vim.log.levels.INFO)
    elseif record_value.event == "watch.build.completed" then
      -- Every completion replaces the complete snapshot, including files
      -- which disappeared from the previous build. Dirty buffers are skipped
      -- by the projection and remain under LSP ownership.
      local replacement_ok, replacement_error = pcall(state.replace_diagnostics,
        record_value.data.diagnostics, session.project_root)
      if not replacement_ok then failure(session, replacement_error); return end
      report("neovim-command-watch-status", { detail = json(record_value.data) }, vim.log.levels.INFO)
    elseif record_value.event == "watch.stopped" then
      session.stopped = true
      if record_value.data.reason.type == "fatal" then
        report("neovim-command-failure", { detail = json(record_value.data.error) }, vim.log.levels.ERROR)
        session.fatal = true
        session.fatal_timer = timer.after(session.timing.stop_timeout_ms, function()
          session.fatal_timer = nil
          if state.active == session and not session.closed then recover(session) end
        end)
      else
        report("neovim-command-watch-status", { detail = json(record_value.data) }, vim.log.levels.INFO)
      end
    else
      report("neovim-command-watch-status", { detail = json(record_value.data) }, vim.log.levels.INFO)
    end
  end

  local function close(session, event)
    if state.active ~= session or session.retired then return end
    session.closed = true
    local ok, error = pcall(session.validator.finish, session.validator, event.code)
    if not ok and not session.failed and not session.fenced then
      report("neovim-command-protocol-failure", { detail = protocol.error_message(error) }, vim.log.levels.ERROR)
    end
    retire(session)
  end

  function adapter.start(options_override)
    if state.disposed then
      report("neovim-command-protocol-failure", { detail = "watch_adapter_disposed" }, vim.log.levels.ERROR)
      return nil
    end
    if state.active and not state.active.retired then
      report("neovim-command-protocol-failure", {
        detail = state.active.hung and "watch_process_hung" or "watch_already_running",
      }, vim.log.levels.ERROR)
      return nil
    end
    local configuration = vim.tbl_deep_extend("force", vim.deepcopy(state.config), options_override or {})
    local root = configuration.project_root
    if not root or root == "" then
      report("neovim-command-input-invalid", {}, vim.log.levels.ERROR)
      return nil
    end
    root = absolute(root)
    if vim.fn.isdirectory(root) ~= 1 then
      report("neovim-command-input-invalid", {}, vim.log.levels.ERROR)
      return nil
    end
    local binary = configuration.binary or "recite"
    if type(binary) == "table" then binary = binary[1] end
    if type(binary) ~= "string" or binary == "" or vim.fn.executable(binary) ~= 1 then
      report("neovim-command-cli-missing", { command = tostring(binary) }, vim.log.levels.ERROR)
      return nil
    end
    state.generation = state.generation + 1
    local invocation_id = options_override and options_override.invocation_id or string.format("nvim-watch-%d-%d", vim.fn.localtime(), state.generation)
    local session = {
      invocation_id = invocation_id,
      validator = watch_protocol.new(invocation_id, root),
      retired = false,
      recovery = false,
      stopped = false,
      closed = false,
      failed = false,
      fenced = false,
      transport = nil,
      timing = timing(configuration),
      stop_timer = nil,
      force_timer = nil,
      recovery_timer = nil,
      fatal_timer = nil,
      hung_timer = nil,
      hung = false,
    }
    state.active = session
    session.transport = process.start_stream({
      argv = { binary, "watch", "--output-format", "structured", "--invocation-id", invocation_id, root },
      cwd = configuration.cwd or root,
      max_bytes = configuration.max_watch_bytes,
      system = configuration.system,
      terminate_on_error = false,
      on_callback_failed = function(error)
        report("neovim-callback-failed", { kind = "watch", detail = tostring(error) }, vim.log.levels.ERROR)
      end,
      on_spawn = function(child) session.child = child end,
      on_record = function(value) record(session, value) end,
      on_error = function(error) failure(session, error) end,
      on_close = function(event) close(session, event) end,
    })
    if session.transport and session.transport.failed then failure(session, protocol.error("spawn")) end
    return { invocation_id = invocation_id }
  end

  function adapter.stop()
    local session = state.active
    if not session or session.retired then
      report("neovim-command-watch-not-running", {}, vim.log.levels.ERROR)
      return false
    end
    if session.stop_requested then return true end
    if not request_cancel(session) then return false end
    session.stop_timer = timer.after(session.timing.stop_timeout_ms, function()
      if state.active ~= session or session.retired or session.closed then return end
      report("neovim-command-watch-stop-timeout", {}, vim.log.levels.ERROR)
      recover(session)
    end)
    return true
  end

  function adapter.active()
    return state.active and not state.active.retired and state.active or nil
  end

  function adapter.reconfigure()
    local session = state.active
    if not session or session.retired then return true end
    session.fenced = true
    state.clear_diagnostics()
    return adapter.stop()
  end

  function adapter.dispose()
    state.disposed = true
    local session = state.active
    if not session then return end
    if not session.stop_requested then adapter.stop() end
  end

  -- VimLeavePre runs immediately before Neovim tears down its event loop.
  -- Give the child the normal cancel handshake first, then synchronously
  -- escalate so a real process cannot outlive the editor by accident.
  function adapter.dispose_sync()
    state.disposed = true
    local session = state.active
    if not session then return true end
    request_cancel(session)
    vim.wait(session.timing.cancel_grace_ms, function()
      return session.closed or session.retired
    end, 10)
    if session.closed or session.retired then return true end
    process.terminate(session.transport, false)
    vim.wait(session.timing.stop_timeout_ms, function()
      return session.closed or session.retired
    end, 10)
    if session.closed or session.retired then return true end
    process.terminate(session.transport, true)
    vim.wait(session.timing.teardown_timeout_ms, function()
      return session.closed or session.retired
    end, 10)
    if not session.closed and not session.retired then
      report("neovim-command-protocol-failure", { detail = "watch_process_hung" }, vim.log.levels.ERROR)
      return false
    end
    return true
  end

  function adapter.configure(config)
    state.config = vim.tbl_deep_extend("force", state.config, config or {})
  end

  return adapter
end

return M
