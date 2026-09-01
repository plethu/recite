-- Generated from crates/recite-ui/resources/en-US.ftl via the canonical inventory.
-- Do not edit. Run editor/recite-neovim/scripts/message-projections.mjs to regenerate.
local templates = {
  ["lsp-client-display-name"] = "Recite",
  ["lsp-client-restart-exhausted"] = "Recite language server restart attempts exhausted.",
  ["neovim-autocmd-description"] = "Start Recite syntax and language tooling",
  ["neovim-callback-failed"] = "Recite {$kind} callback failed: {$detail}",
  ["neovim-health-filetype-ok"] = ".recite files use the recite filetype",
  ["neovim-health-filetype-error"] = ".recite filetype registration is unavailable",
  ["neovim-health-lsp-executable-found"] = "language server executable found: {$command}",
  ["neovim-health-lsp-executable-missing"] = "language server executable not found: {$command}",
  ["neovim-health-lsp-install"] = "Install recite-lsp or set lsp.cmd in require('recite').setup",
  ["neovim-health-query-found"] = "Tree-sitter highlight query is on runtimepath",
  ["neovim-health-query-missing"] = "Tree-sitter highlight query is not on runtimepath",
  ["neovim-health-parser-found"] = "Tree-sitter parser is on runtimepath",
  ["neovim-health-parser-missing"] = "Tree-sitter parser is not built or is not on runtimepath",
  ["neovim-health-parser-build"] = "Build it from editor/recite-tree-sitter; see editor/recite-neovim/README.md",
  ["neovim-health-current-root"] = "current project root: {$root}",
  ["neovim-health-open-buffer"] = "Open a .recite buffer to inspect its project root",
}

local M = {}

function M.format(id, arguments)
  local template = templates[id]
  if template == nil then
    error("unknown Recite UI message: " .. tostring(id), 2)
  end
  arguments = arguments or {}
  return (template:gsub("{%$([%w_]+)}", function(name)
    local value = arguments[name]
    if value == nil then
      error("missing argument for Recite UI message " .. id .. ".$" .. name, 2)
    end
    return tostring(value)
  end))
end

return M
