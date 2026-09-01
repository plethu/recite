local M = {}
local messages = require("recite_messages")

local RESTART_LIMIT = 3
local RESTART_STABILITY_MS = 1000

local defaults = {
  lsp = {
    autostart = true,
    cmd = { "recite-lsp" },
    root_markers = { "recite.project.toml" },
  },
  treesitter = {
    enabled = true,
  },
}

local state = {
  augroup = nil,
  config = vim.deepcopy(defaults),
  clients = {},
  pending_restarts = {},
  restart_attempts = {},
  restart_generation = 0,
}

local function stop_timer(timer)
  if type(timer) == "number" then
    vim.fn.timer_stop(timer)
  elseif timer then
    pcall(timer.stop, timer)
  end
end

local function join_path(parent, child)
  if vim.fs and vim.fs.joinpath then
    return vim.fs.joinpath(parent, child)
  end
  return parent .. "/" .. child
end

local function normalise(path)
  if vim.fs and vim.fs.normalize then
    return vim.fs.normalize(path)
  end
  return path
end

local function containing_directory(bufnr)
  local name = vim.api.nvim_buf_get_name(bufnr)
  if name == "" then
    return normalise(vim.fn.getcwd())
  end
  return normalise(vim.fn.fnamemodify(name, ":p:h"))
end

local function parent_directory(path)
  local parent = vim.fn.fnamemodify(path, ":h")
  if parent == path then
    return nil
  end
  return normalise(parent)
end

local function has_file(path)
  local fs = vim.uv or vim.loop
  local stat = fs and fs.fs_stat(path)
  return stat and stat.type == "file"
end

local function find_marker_root(start, markers)
  local current = start
  while current do
    for _, marker in ipairs(markers) do
      if has_file(join_path(current, marker)) then
        return current
      end
    end
    current = parent_directory(current)
  end
end

local function start_treesitter(bufnr)
  if not state.config.treesitter.enabled then
    return
  end
  if not (vim.treesitter and vim.treesitter.start) then
    return
  end
  -- A parser is an optional separately-built artifact.  Avoid making opening
  -- a .recite buffer fail when a user has only installed the LSP integration.
  pcall(vim.treesitter.start, bufnr, "recite")
end

local function attach(bufnr)
  start_treesitter(bufnr)
  if state.config.lsp.autostart then
    M.start(bufnr)
  end
end

local function replace_config(options)
  local config = vim.deepcopy(state.config or defaults)
  if options then
    config = vim.tbl_deep_extend("force", config, options)
  end
  return config
end

local function stop_owned_clients()
  for lifecycle in pairs(state.pending_restarts) do
    lifecycle.intentional = true
    if lifecycle.timer then
      stop_timer(lifecycle.timer)
      lifecycle.timer = nil
    end
    if lifecycle.stability_timer then
      stop_timer(lifecycle.stability_timer)
      lifecycle.stability_timer = nil
    end
    state.pending_restarts[lifecycle] = nil
  end

  for client_id, lifecycle in pairs(state.clients) do
    lifecycle.intentional = true
    state.restart_attempts[lifecycle.root] = 0
    if lifecycle.timer then
      stop_timer(lifecycle.timer)
      lifecycle.timer = nil
    end
    if lifecycle.stability_timer then
      stop_timer(lifecycle.stability_timer)
      lifecycle.stability_timer = nil
    end
    local client = vim.lsp.get_client_by_id(client_id)
    if client then
      client:stop(true)
    end
    state.clients[client_id] = nil
  end
end

local function report_callback_error(kind, callback_error)
  -- Callback failures belong to the caller, but they must remain visible when
  -- the integration recovers its own lifecycle.  Keep reporting best effort:
  -- a notification backend is also caller-provided code.
  local message = messages.format("neovim-callback-failed", {
    kind = kind,
    detail = callback_error,
  })
  pcall(vim.notify, message, vim.log.levels.ERROR)
end

local function report_restart_exhausted()
  pcall(vim.notify, messages.format("lsp-client-restart-exhausted"), vim.log.levels.ERROR)
end

local function invoke_callback(kind, callback, ...)
  if not callback then
    return
  end
  local ok, callback_error = xpcall(callback, debug.traceback, ...)
  if not ok then
    return callback_error
  end
end

local function configured_root(bufnr, lsp)
  local root = lsp.root_dir
  if type(root) == "function" then
    root = root(bufnr)
  end
  return root or M.root_dir(bufnr, lsp.root_markers)
end

local function has_open_buffer(root, lsp)
  local buffers = {}
  for _, bufnr in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_is_valid(bufnr)
      and vim.bo[bufnr].buflisted
      and vim.bo[bufnr].filetype == "recite"
      and configured_root(bufnr, lsp or state.config.lsp) == root then
      buffers[#buffers + 1] = bufnr
    end
  end
  return buffers
end

local function schedule_restart(lifecycle)
  local generation = lifecycle.generation
  vim.schedule(function()
    if lifecycle.intentional or generation ~= state.restart_generation then
      return
    end
    local buffers = has_open_buffer(lifecycle.root, state.config.lsp)
    if #buffers == 0 then
      return
    end
    if lifecycle.attempts >= RESTART_LIMIT then
      report_restart_exhausted()
      return
    end
    lifecycle.attempts = lifecycle.attempts + 1
    state.restart_attempts[lifecycle.root] = lifecycle.attempts
    local delay = math.min(2000, 100 * (2 ^ (lifecycle.attempts - 1)))
    lifecycle.timer = vim.defer_fn(function()
      lifecycle.timer = nil
      state.pending_restarts[lifecycle] = nil
      vim.schedule(function()
        if lifecycle.intentional or generation ~= state.restart_generation then
          return
        end
        for _, bufnr in ipairs(has_open_buffer(lifecycle.root, state.config.lsp)) do
          M.start(bufnr)
        end
      end)
    end, delay)
    state.pending_restarts[lifecycle] = true
  end)
end

--- Return the deterministic project root used for a buffer.
---
--- The nearest exact `recite.project.toml` wins.  A source-only workspace with
--- no manifest uses the buffer's containing directory; callers with a larger
--- source-only project can provide `lsp.root_dir` explicitly.
function M.root_dir(bufnr, markers)
  bufnr = bufnr or 0
  markers = markers or state.config.lsp.root_markers
  local directory = containing_directory(bufnr)
  return find_marker_root(directory, markers) or directory
end

--- Return the configured language-server command as a copy safe to customise.
function M.command()
  return vim.deepcopy(state.config.lsp.cmd)
end

--- Start or reuse Recite's language server for a buffer.
function M.start(bufnr, overrides)
  bufnr = bufnr or 0
  local lsp = vim.deepcopy(state.config.lsp)
  if overrides then
    lsp = vim.tbl_deep_extend("force", lsp, overrides)
  end

  local root = configured_root(bufnr, lsp)

  local client_config = {
    name = lsp.name or "recite-lsp",
    cmd = lsp.cmd,
    root_dir = root,
    recite_owned = true,
  }
  -- Do not invent capabilities or keymaps.  Neovim's defaults are used unless
  -- the caller explicitly supplies a capability table or attach callback.
  for _, key in ipairs({ "capabilities", "init_options", "on_attach", "settings" }) do
    if lsp[key] ~= nil then
      client_config[key] = vim.deepcopy(lsp[key])
    end
  end

  local caller_on_exit = lsp.on_exit
  client_config.on_exit = function(code, signal, exited_id)
    vim.schedule(function()
      local callback_error = invoke_callback("on_exit", caller_on_exit, code, signal, exited_id)
      local lifecycle = state.clients[exited_id]
      if lifecycle then
        if lifecycle.stability_timer then
          stop_timer(lifecycle.stability_timer)
          lifecycle.stability_timer = nil
        end
        state.clients[exited_id] = nil
        if not lifecycle.intentional then
          schedule_restart(lifecycle)
        end
      end
      if callback_error then
        report_callback_error("on_exit", callback_error)
      end
    end)
  end

  local caller_on_init = lsp.on_init
  client_config.on_init = function(initialized_client, initialize_result)
    local lifecycle = state.clients[initialized_client.id]
    if lifecycle then
      -- A successful initialize is not proof that the process is stable. Keep
      -- the bounded crash budget until the client has remained alive for the
      -- stability window, so rapid initialize-then-crash loops eventually
      -- surface an exhausted recovery state.
      if lifecycle.stability_timer then
        stop_timer(lifecycle.stability_timer)
      end
      lifecycle.stability_timer = vim.defer_fn(function()
        lifecycle.stability_timer = nil
        if lifecycle.intentional or lifecycle.generation ~= state.restart_generation then
          return
        end
        local active = vim.lsp.get_client_by_id(initialized_client.id)
        if active and active.config.recite_owned == true then
          lifecycle.attempts = 0
          state.restart_attempts[root] = 0
        end
      end, RESTART_STABILITY_MS)
    end
    local callback_error = invoke_callback("on_init", caller_on_init, initialized_client, initialize_result)
    if callback_error then
      report_callback_error("on_init", callback_error)
    end
  end

  local client_id = vim.lsp.start(client_config, {
    bufnr = bufnr,
    reuse_client = function(client, config)
      return client.config.recite_owned == true
        and client.config.root_dir == config.root_dir
    end,
  })
  if client_id then
    local client = vim.lsp.get_client_by_id(client_id)
    if client and client.config.recite_owned == true then
      -- `vim.lsp.start` may reuse an active owned client. Preserve its
      -- lifecycle record in that case: replacing it would orphan the
      -- stability timer that eventually resets the bounded crash budget.
      if not state.clients[client_id] then
        state.clients[client_id] = {
          generation = state.restart_generation,
          root = root,
          intentional = false,
          stability_timer = nil,
          attempts = state.restart_attempts[root] or 0,
        }
      end
    end
  end
  return client_id
end

--- Stop a Recite client without scheduling crash recovery.
function M.stop(client_id)
  local lifecycle = state.clients[client_id]
  if not lifecycle then
    return false
  end
  lifecycle.intentional = true
  if lifecycle.timer then
    stop_timer(lifecycle.timer)
    lifecycle.timer = nil
  end
  if lifecycle.stability_timer then
    stop_timer(lifecycle.stability_timer)
    lifecycle.stability_timer = nil
  end
  local client = vim.lsp.get_client_by_id(client_id)
  if client then
    client:stop(true)
  end
  return true
end

--- Register filetype, Tree-sitter, and FileType integration.
function M.setup(options)
  local next_config = replace_config(options)
  local config_changed = not vim.deep_equal(next_config, state.config)
  if config_changed then
    state.restart_generation = state.restart_generation + 1
    stop_owned_clients()
    state.restart_attempts = {}
  end
  state.config = next_config
  vim.g.recite_setup = true

  vim.filetype.add({ extension = { recite = "recite" } })
  if vim.treesitter and vim.treesitter.language then
    vim.treesitter.language.register("recite", "recite")
  end

  if state.augroup then
    vim.api.nvim_del_augroup_by_id(state.augroup)
  end
  state.augroup = vim.api.nvim_create_augroup("recite_editor", { clear = true })
  vim.api.nvim_create_autocmd("FileType", {
    group = state.augroup,
    pattern = "recite",
    callback = function(args)
      attach(args.buf)
    end,
    desc = messages.format("neovim-autocmd-description"),
  })

  -- `setup` is also safe to call from an init.lua after a buffer already
  -- exists.  Only existing Recite buffers are attached; no other filetype is
  -- touched.
  for _, bufnr in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_is_valid(bufnr) and vim.bo[bufnr].filetype == "recite" then
      attach(bufnr)
    end
  end
  return M
end

return M
