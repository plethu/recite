local messages = require("recite_messages")
local material = require("recite.material")

local RESTART_LIMIT = 3
local RESTART_STABILITY_MS = 1000
local function new(options)
  local state = options.state
  local resolve_root = options.root_dir
  local api = {}

  local function stop_timer(timer)
    if type(timer) == "number" then
      vim.fn.timer_stop(timer)
    elseif timer then
      pcall(timer.stop, timer)
    end
  end

  local function configured_root(bufnr, lsp)
    local root = lsp.root_dir
    if type(root) == "function" then
      root = root(bufnr)
    end
    return root or resolve_root(bufnr, lsp.root_markers)
  end

  local function open_buffers(root, lsp)
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

  local function report_callback_error(kind, detail)
    pcall(vim.notify, messages.format("neovim-callback-failed", {
      kind = kind,
      detail = detail,
    }), vim.log.levels.ERROR)
  end

  local function report_exhausted()
    pcall(vim.notify, messages.format("lsp-client-restart-exhausted"), vim.log.levels.ERROR)
  end

  local function invoke(kind, callback, ...)
    if not callback then
      return
    end
    local ok, detail = xpcall(callback, debug.traceback, ...)
    if not ok then
      return detail
    end
  end

  local start_client
  local function schedule_restart(lifecycle)
    local generation = lifecycle.generation
    vim.schedule(function()
      if lifecycle.intentional or generation ~= state.restart_generation then
        return
      end
      if #open_buffers(lifecycle.root, material.root_config(lifecycle, state.config.lsp)) == 0 then
        return
      end
      if lifecycle.attempts >= RESTART_LIMIT then
        report_exhausted()
        return
      end
      lifecycle.attempts = lifecycle.attempts + 1
      local delay = math.min(2000, 100 * (2 ^ (lifecycle.attempts - 1)))
      lifecycle.timer = vim.defer_fn(function()
        lifecycle.timer = nil
        state.pending_restarts[lifecycle] = nil
        vim.schedule(function()
          if lifecycle.intentional or generation ~= state.restart_generation then
            return
          end
          for _, bufnr in ipairs(open_buffers(lifecycle.root, material.root_config(lifecycle, state.config.lsp))) do
            start_client(bufnr, material.restart_overrides(lifecycle.material), lifecycle.attempts)
          end
        end)
      end, delay)
      state.pending_restarts[lifecycle] = true
    end)
  end

  function api.stop_owned_clients()
    for lifecycle in pairs(state.pending_restarts) do
      lifecycle.intentional = true
      stop_timer(lifecycle.timer)
      stop_timer(lifecycle.stability_timer)
      lifecycle.timer = nil
      lifecycle.stability_timer = nil
      state.pending_restarts[lifecycle] = nil
    end
    for client_id, lifecycle in pairs(state.clients) do
      lifecycle.intentional = true
      stop_timer(lifecycle.timer)
      stop_timer(lifecycle.stability_timer)
      lifecycle.timer = nil
      lifecycle.stability_timer = nil
      local client = vim.lsp.get_client_by_id(client_id)
      if client then
        client:stop(true)
      end
      state.clients[client_id] = nil
    end
  end

  start_client = function(bufnr, overrides, retry_attempts)
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
    for _, key in ipairs({ "capabilities", "init_options", "on_attach", "settings" }) do
      if lsp[key] ~= nil then
        client_config[key] = vim.deepcopy(lsp[key])
      end
    end

    local caller_on_exit = lsp.on_exit
    client_config.on_exit = function(code, signal, exited_id)
      vim.schedule(function()
        local detail = invoke("on_exit", caller_on_exit, code, signal, exited_id)
        local lifecycle = state.clients[exited_id]
        if lifecycle then
          stop_timer(lifecycle.stability_timer)
          lifecycle.stability_timer = nil
          state.clients[exited_id] = nil
          if not lifecycle.intentional then
            schedule_restart(lifecycle)
          end
        end
        if detail then
          report_callback_error("on_exit", detail)
        end
      end)
    end

    local caller_on_init = lsp.on_init
    client_config.on_init = function(initialized_client, initialize_result)
      local lifecycle = state.clients[initialized_client.id]
      if lifecycle then
        stop_timer(lifecycle.stability_timer)
        lifecycle.stability_timer = vim.defer_fn(function()
          lifecycle.stability_timer = nil
          if lifecycle.intentional or lifecycle.generation ~= state.restart_generation then
            return
          end
          local active = vim.lsp.get_client_by_id(initialized_client.id)
          if active and active.config.recite_owned == true then
            lifecycle.attempts = 0
          end
        end, RESTART_STABILITY_MS)
      end
      local detail = invoke("on_init", caller_on_init, initialized_client, initialize_result)
      if detail then
        report_callback_error("on_init", detail)
      end
    end

    client_config.recite_material = {
      name = client_config.name,
      root_dir = root,
      root_dir_spec = lsp.root_dir,
      root_markers = vim.deepcopy(lsp.root_markers),
      cmd = vim.deepcopy(client_config.cmd),
      settings = vim.deepcopy(client_config.settings),
      init_options = vim.deepcopy(client_config.init_options),
      capabilities = vim.deepcopy(client_config.capabilities),
      on_attach = client_config.on_attach,
      on_init = caller_on_init,
      on_exit = caller_on_exit,
    }

    local client_id = vim.lsp.start(client_config, {
      bufnr = bufnr,
      reuse_client = function(client, config)
        return client.config.recite_owned == true
          and client.config.root_dir == config.root_dir
          and material.same(client, config)
      end,
    })
    if client_id then
      local client = vim.lsp.get_client_by_id(client_id)
      if client and client.config.recite_owned == true and not state.clients[client_id] then
        state.clients[client_id] = {
          generation = state.restart_generation,
          root = root,
          intentional = false,
          stability_timer = nil,
          attempts = retry_attempts or 0,
          material = vim.deepcopy(client.config.recite_material),
        }
      end
    end
    return client_id
  end

  function api.start(bufnr, overrides)
    return start_client(bufnr, overrides, 0)
  end

  function api.stop(client_id)
    local lifecycle = state.clients[client_id]
    if not lifecycle then
      return false
    end
    lifecycle.intentional = true
    stop_timer(lifecycle.timer)
    stop_timer(lifecycle.stability_timer)
    lifecycle.timer = nil
    lifecycle.stability_timer = nil
    local client = vim.lsp.get_client_by_id(client_id)
    if client then
      client:stop(true)
    end
    return true
  end

  return api
end

return { new = new }
