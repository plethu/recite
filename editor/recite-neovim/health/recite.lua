local M = {}
local messages = require("recite_messages")

local function health_api()
  if vim.health then
    return vim.health
  end
  return require("health")
end

local function command_name(command)
  if type(command) == "string" then
    return command
  end
  return command[1]
end

function M.check()
  local health = health_api()
  health.start(messages.format("lsp-client-display-name"))

  local ok, filetype = pcall(vim.filetype.match, { filename = "health.recite" })
  if ok and filetype == "recite" then
    health.ok(messages.format("neovim-health-filetype-ok"))
  else
    health.error(messages.format("neovim-health-filetype-error"))
  end

  local config = require("recite")
  local command = command_name(config.command())
  if vim.fn.executable(command) == 1 then
    health.ok(messages.format("neovim-health-lsp-executable-found", { command = command }))
  else
    health.warn(messages.format("neovim-health-lsp-executable-missing", { command = command }))
    health.info(messages.format("neovim-health-lsp-install"))
  end

  local query = vim.api.nvim_get_runtime_file("queries/recite/highlights.scm", true)
  if #query > 0 then
    health.ok(messages.format("neovim-health-query-found"))
  else
    health.warn(messages.format("neovim-health-query-missing"))
  end

  local parser = vim.api.nvim_get_runtime_file("parser/recite.*", true)
  if #parser > 0 then
    health.ok(messages.format("neovim-health-parser-found"))
  else
    health.warn(messages.format("neovim-health-parser-missing"))
    health.info(messages.format("neovim-health-parser-build"))
  end

  local bufnr = vim.api.nvim_get_current_buf()
  if vim.bo[bufnr].filetype == "recite" then
    health.ok(messages.format("neovim-health-current-root", { root = config.root_dir(bufnr) }))
  else
    health.info(messages.format("neovim-health-open-buffer"))
  end
end

return M
