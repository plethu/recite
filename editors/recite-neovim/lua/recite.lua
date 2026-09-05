local M = {}
local messages = require("recite_messages")

local defaults = {
  lsp = {
    autostart = true,
    cmd = { "recite-lsp" },
    root_markers = { "recite.project.toml" },
  },
  treesitter = {
    enabled = true,
  },
  commands = {
    binary = "recite",
    max_finite_bytes = 32 * 1024 * 1024,
    max_watch_bytes = 4 * 1024 * 1024,
  },
}

local state = {
  augroup = nil,
  config = vim.deepcopy(defaults),
  clients = {},
  pending_restarts = {},
  restart_generation = 0,
}
local lsp_lifecycle
local command_adapter

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

--- Return the deterministic project root used for a buffer.
function M.root_dir(bufnr, markers)
  bufnr = bufnr or 0
  markers = markers or state.config.lsp.root_markers
  local directory = containing_directory(bufnr)
  return find_marker_root(directory, markers) or directory
end

lsp_lifecycle = require("recite.lifecycle").new({
  state = state,
  root_dir = function(bufnr, markers)
    return M.root_dir(bufnr, markers)
  end,
})
command_adapter = require("recite.commands").new({
  config = state.config.commands,
  root_dir = function(bufnr)
    return M.root_dir(bufnr)
  end,
})

--- Start or reuse Recite's language server for a buffer.
function M.start(bufnr, overrides)
  return lsp_lifecycle.start(bufnr, overrides)
end

--- Stop a Recite client without scheduling crash recovery.
function M.stop(client_id)
  return lsp_lifecycle.stop(client_id)
end

--- Return the configured language-server command as a copy safe to customise.
function M.command()
  return vim.deepcopy(state.config.lsp.cmd)
end

-- Structured CLI adapters have a lifecycle separate from LSP.
if vim.env.RECITE_DISABLE_COMMANDS ~= "1" then
  M.commands = command_adapter
  M.validate = function(options) return command_adapter.validate(options) end
  M.compile = function(options) return command_adapter.compile(options) end
  M.extract = function(options) return command_adapter.extract(options) end
  M.run = function(options) return command_adapter.run(options) end
  M.trace = function(options) return command_adapter.trace(options) end
  M.watch_start = function(options) return command_adapter.watch_start(options) end
  M.watch_stop = function() return command_adapter.watch_stop() end
  M.watch_active = function() return command_adapter.watch_active() end
end

--- Register filetype, Tree-sitter, and FileType integration.
function M.setup(options)
  local next_config = replace_config(options)
  local config_changed = not vim.deep_equal(next_config, state.config)
  if config_changed then
    state.restart_generation = state.restart_generation + 1
    lsp_lifecycle.stop_owned_clients()
  end
  state.config = next_config
  command_adapter.configure(state.config.commands)
  vim.g.recite_setup = true

  vim.filetype.add({ extension = { recite = "recite" } })
  if vim.treesitter and vim.treesitter.language then
    vim.treesitter.language.register("recite", "recite")
  end

  if state.augroup then
    vim.api.nvim_del_augroup_by_id(state.augroup)
  end

  local function command(name, callback, command_options)
    if vim.fn.exists(":" .. name) == 2 then vim.api.nvim_del_user_command(name) end
    local definition = vim.tbl_extend("force", {
      nargs = "*",
      complete = "file",
      desc = messages.format("neovim-command-description", { command = name }),
    }, command_options or {})
    if definition.complete == false then definition.complete = nil end
    vim.api.nvim_create_user_command(name, callback, definition)
  end
  command("ReciteValidate", function(args)
    local values = args.fargs
    M.validate(#values > 0 and { paths = values } or {})
  end)
  command("ReciteCompile", function(args)
    local values = args.fargs
    M.compile(#values > 1 and { output = values[1], paths = vim.list_slice(values, 2) }
      or #values == 1 and { output = values[1] } or {})
  end)
  command("ReciteExtract", function(args)
    local values = args.fargs
    M.extract(#values > 1 and { output = values[1], paths = vim.list_slice(values, 2) }
      or #values == 1 and { output = values[1] } or {})
  end)
  command("ReciteRun", function(args)
    local values = args.fargs
    if #values ~= 3 then
      vim.notify(messages.format("neovim-command-input-invalid"), vim.log.levels.ERROR)
      return
    end
    M.run({ asset = values[1], block = values[2], fixture = values[3] })
  end)
  command("ReciteTrace", function(args)
    local values = args.fargs
    if #values ~= 3 then
      vim.notify(messages.format("neovim-command-input-invalid"), vim.log.levels.ERROR)
      return
    end
    M.trace({ asset = values[1], block = values[2], fixture = values[3] })
  end)
  command("ReciteWatchStart", function(args)
    local values = args.fargs
    M.watch_start(#values > 0 and { project_root = values[1] } or {})
  end, { nargs = "?" })
  command("ReciteWatchStop", function()
    M.watch_stop()
  end, { nargs = 0, complete = false })
  state.augroup = vim.api.nvim_create_augroup("recite_editor", { clear = true })
  vim.api.nvim_create_autocmd("FileType", {
    group = state.augroup,
    pattern = "recite",
    callback = function(args)
      attach(args.buf)
    end,
    desc = messages.format("neovim-autocmd-description"),
  })
  vim.api.nvim_create_autocmd("VimLeavePre", {
    group = state.augroup,
    callback = function()
      command_adapter.dispose_sync()
      lsp_lifecycle.stop_owned_clients()
    end,
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
