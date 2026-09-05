local protocol = require("recite.command_protocol")
local finite_protocol = require("recite.finite_protocol")
local timer = require("recite.timer")

local M = {}

local function callback_failed(options, error)
  if options.on_callback_failed then pcall(options.on_callback_failed, error) end
end

local function invoke(options, callback, ...)
  if not callback then return true end
  local ok, error = pcall(callback, ...)
  if not ok then callback_failed(options, error) end
  return ok, error
end

local function schedule(options, callback, ...)
  if not callback then return end
  local arguments = { ... }
  vim.schedule(function() invoke(options, callback, unpack(arguments)) end)
end

local function terminate(session, force)
  if not session.process or session.closed then return end
  pcall(session.process.kill, session.process, force and "sigkill" or "sigterm")
end

local function cancel_timers(session)
  timer.stop(session.term_timer)
  timer.stop(session.kill_timer)
  session.term_timer, session.kill_timer = nil, nil
end

local function terminate_bounded(session, term_timeout_ms, kill_timeout_ms, on_hung)
  if session.closed or session.termination_started then return end
  session.termination_started = true
  terminate(session, false)
  session.term_timer = timer.after(term_timeout_ms or 250, function()
    session.term_timer = nil
    if session.closed then return end
    terminate(session, true)
    session.kill_timer = timer.after(kill_timeout_ms or 250, function()
      session.kill_timer = nil
      if session.closed then return end
      session.hung = true
      if on_hung then on_hung(session) end
    end)
  end)
end

local function close_once(session, callback, event, options)
  if session.close_called then return end
  session.close_called = true
  invoke(options, callback, event)
end

local function make_system(options, on_exit)
  local system = options.system or vim.system
  return system(options.argv, {
    cwd = options.cwd,
    stdin = options.stdin == false and nil or true,
    text = false,
    stdout = options.stdout,
    stderr = options.stderr,
  }, on_exit)
end

-- A finite invocation owns one process and exactly one result/error callback.
-- Its generation is supplied by the command adapter; this layer only handles
-- bytes, process ownership, and protocol framing.
function M.start_finite(options)
  local session = {
    argv = vim.deepcopy(options.argv),
    cwd = options.cwd,
    parser = protocol.new_parser(options.max_record_bytes or protocol.MAX_RECORD_BYTES, options.max_bytes or protocol.MAX_FINITE_BYTES),
    records = {},
    bytes = 0,
    stderr_bytes = 0,
    failed = false,
    finished = false,
    closed = false,
    process = nil,
    term_timer = nil,
    kill_timer = nil,
    termination_started = false,
    hung = false,
  }

  local function report_error(error)
    if session.error_called then return end
    session.error_called = true
    schedule(options, options.on_error, error)
  end

  local function fail(error)
    if session.failed or session.finished then return end
    session.failed = true
    terminate_bounded(session, options.term_timeout_ms, options.kill_timeout_ms, options.on_hung)
    report_error(error)
  end

  local function stdout(_, data)
    if session.failed or session.closed or not data then return end
    session.bytes = session.bytes + #data
    if session.bytes > (options.max_bytes or protocol.MAX_FINITE_BYTES) then
      fail(protocol.error("stdout_too_large"))
      return
    end
    local ok, records = pcall(session.parser.push, session.parser, data)
    if not ok then
      fail(records)
      return
    end
    vim.list_extend(session.records, records)
  end

  local function stderr(_, data)
    if session.failed or session.closed or not data or #data == 0 then return end
    session.stderr_bytes = session.stderr_bytes + #data
    fail(protocol.error(session.stderr_bytes > protocol.MAX_STDERR_BYTES
      and "stderr_too_large" or "stderr_output"))
  end

  local function on_exit(result)
    if session.closed then return end
    session.closed = true
    cancel_timers(session)
    if session.failed then
      schedule(options, close_once, session, options.on_close, { failed = true, code = result and result.code, signal = result and result.signal }, options)
      return
    end
    schedule(options, function()
      local ok, error = pcall(session.parser.finish, session.parser)
      if not ok then
        session.failed = true
        report_error(error)
        close_once(session, options.on_close, { failed = true, code = result and result.code, signal = result and result.signal }, options)
        return
      end
      local exit_code = result and result.code
      local parsed_ok, parsed = pcall(finite_protocol.parse, session.records, options.command, options.invocation_id, exit_code)
      if not parsed_ok then
        session.failed = true
        report_error(parsed)
        close_once(session, options.on_close, { failed = true, code = exit_code, signal = result and result.signal }, options)
        return
      end
      local callback_ok, callback_error = invoke(options, options.on_result, parsed, { code = exit_code, signal = result and result.signal })
      if not callback_ok then
        session.failed = true
        report_error(protocol.error("callback_failed", tostring(callback_error)))
        terminate_bounded(session, options.term_timeout_ms, options.kill_timeout_ms, options.on_hung)
      end
      close_once(session, options.on_close, { failed = session.failed, code = exit_code, signal = result and result.signal }, options)
    end)
  end

  local ok, process = pcall(make_system, vim.tbl_extend("force", options, { stdout = stdout, stderr = stderr }), on_exit)
  if not ok then
    session.failed = true
    report_error(protocol.error("spawn", tostring(process)))
    session.closed = true
    schedule(options, close_once, session, options.on_close, { failed = true }, options)
    return session
  end
  session.process = process
  invoke(options, options.on_spawn, process, session)
  return session
end

-- The streaming variant deliberately leaves lifecycle state to the watch
-- validator. It provides only bounded NDJSON chunks, stderr refusal, and
-- explicit write/TERM/KILL operations.
function M.start_stream(options)
  local session = {
    argv = vim.deepcopy(options.argv),
    cwd = options.cwd,
    parser = protocol.new_parser(options.max_bytes or protocol.MAX_RECORD_BYTES),
    bytes = 0,
    stderr_bytes = 0,
    failed = false,
    finished = false,
    closed = false,
    process = nil,
    term_timer = nil,
    kill_timer = nil,
    termination_started = false,
    hung = false,
  }

  local function report_error(error)
    if session.error_called then return end
    session.error_called = true
    schedule(options, options.on_error, error)
  end

  local function fail(error)
    if session.failed or session.closed then return end
    session.failed = true
    report_error(error)
    if options.terminate_on_error ~= false then
      terminate_bounded(session, options.term_timeout_ms, options.kill_timeout_ms, options.on_hung)
    end
  end
  local function stdout(_, data)
    if session.failed or session.closed or not data then return end
    local ok, records = pcall(session.parser.push, session.parser, data)
    if not ok then fail(records); return end
    for _, record in ipairs(records) do
      schedule(options, function()
        if session.failed or session.closed then return end
        local callback_ok, callback_error = invoke(options, options.on_record, record)
        if not callback_ok then fail(protocol.error("callback_failed", tostring(callback_error))) end
      end)
    end
  end
  local function stderr(_, data)
    if session.failed or session.closed or not data or #data == 0 then return end
    session.stderr_bytes = session.stderr_bytes + #data
    fail(protocol.error(session.stderr_bytes > protocol.MAX_STDERR_BYTES
      and "stderr_too_large" or "stderr_output"))
  end
  local function on_exit(result)
    if session.closed then return end
    session.closed = true
    cancel_timers(session)
    local ok, error = pcall(session.parser.finish, session.parser)
    if not ok and not session.failed then
      session.failed = true
      report_error(error)
    end
    schedule(options, close_once, session, options.on_close, {
      failed = session.failed,
      code = result and result.code,
      signal = result and result.signal,
    }, options)
  end
  local ok, process = pcall(make_system, {
    argv = options.argv,
    cwd = options.cwd,
    stdin = true,
    system = options.system,
    stdout = stdout,
    stderr = stderr,
  }, on_exit)
  if not ok then
    session.failed = true
    report_error(protocol.error("spawn", tostring(process)))
    session.closed = true
    schedule(options, close_once, session, options.on_close, { failed = true }, options)
    return session
  end
  session.process = process
  invoke(options, options.on_spawn, process, session)
  return session
end

function M.write(session, data)
  if not session or not session.process or session.closed or session.failed then return false end
  local ok = pcall(session.process.write, session.process, data)
  return ok
end

function M.terminate(session, force)
  terminate(session, force)
end

function M.terminate_bounded(session, term_timeout_ms, kill_timeout_ms, on_hung)
  terminate_bounded(session, term_timeout_ms, kill_timeout_ms, on_hung)
end

return M
