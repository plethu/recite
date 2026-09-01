local M = {}

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
  health.start("Recite")

  local ok, filetype = pcall(vim.filetype.match, { filename = "health.recite" })
  if ok and filetype == "recite" then
    health.ok(".recite files use the recite filetype")
  else
    health.error(".recite filetype registration is unavailable")
  end

  local config = require("recite")
  local command = command_name(config.command())
  if vim.fn.executable(command) == 1 then
    health.ok(("language server executable found: %s"):format(command))
  else
    health.warn(("language server executable not found: %s"):format(command))
    health.info("Install recite-lsp or set lsp.cmd in require('recite').setup")
  end

  local query = vim.api.nvim_get_runtime_file("queries/recite/highlights.scm", true)
  if #query > 0 then
    health.ok("Tree-sitter highlight query is on runtimepath")
  else
    health.warn("Tree-sitter highlight query is not on runtimepath")
  end

  local parser = vim.api.nvim_get_runtime_file("parser/recite.*", true)
  if #parser > 0 then
    health.ok("Tree-sitter parser is on runtimepath")
  else
    health.warn("Tree-sitter parser is not built or is not on runtimepath")
    health.info("Build it from editor/recite-tree-sitter; see editor/recite-neovim/README.md")
  end

  local bufnr = vim.api.nvim_get_current_buf()
  if vim.bo[bufnr].filetype == "recite" then
    health.ok(("current project root: %s"):format(config.root_dir(bufnr)))
  else
    health.info("Open a .recite buffer to inspect its project root")
  end
end

return M
