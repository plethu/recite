local M = {}

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
  restart_attempts = {},
}

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
  for client_id, lifecycle in pairs(state.clients) do
    lifecycle.intentional = true
    state.restart_attempts[lifecycle.root] = 0
    if lifecycle.timer then
      vim.fn.timer_stop(lifecycle.timer)
      lifecycle.timer = nil
    end
    local client = vim.lsp.get_client_by_id(client_id)
    if client then
      client:stop(true)
    end
    state.clients[client_id] = nil
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
  vim.schedule(function()
    if lifecycle.intentional or lifecycle.attempts >= 3 then
      return
    end
    local buffers = has_open_buffer(lifecycle.root, state.config.lsp)
    if #buffers == 0 then
      return
    end
    lifecycle.attempts = lifecycle.attempts + 1
    state.restart_attempts[lifecycle.root] = lifecycle.attempts
    local delay = math.min(2_000, 100 * (2 ^ (lifecycle.attempts - 1)))
    lifecycle.timer = vim.defer_fn(function()
      lifecycle.timer = nil
      vim.schedule(function()
        if lifecycle.intentional then
          return
        end
        local restarted = {}
        for _, bufnr in ipairs(has_open_buffer(lifecycle.root, state.config.lsp)) do
          local client_id = M.start(bufnr)
          if client_id then
            restarted[client_id] = true
          end
        end
        for client_id in pairs(restarted) do
          vim.defer_fn(function()
            vim.schedule(function()
              local client = vim.lsp.get_client_by_id(client_id)
              if client and client.initialized and not client:is_stopped() then
                state.restart_attempts[lifecycle.root] = 0
                local current = state.clients[client_id]
                if current then
                  current.attempts = 0
                end
              end
            end)
          end, 1_000)
        end
      end)
    end, delay)
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
      if caller_on_exit then
        caller_on_exit(code, signal, exited_id)
      end
      local lifecycle = state.clients[exited_id]
      if lifecycle then
        state.clients[exited_id] = nil
        if not lifecycle.intentional then
          schedule_restart(lifecycle)
        end
      end
    end)
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
      state.clients[client_id] = {
        root = root,
        intentional = false,
        attempts = state.restart_attempts[root] or 0,
      }
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
    vim.fn.timer_stop(lifecycle.timer)
    lifecycle.timer = nil
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
    stop_owned_clients()
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
    desc = "Start Recite syntax and language tooling",
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
