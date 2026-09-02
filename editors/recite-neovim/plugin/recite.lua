-- Recite's Neovim integration is a normal runtimepath plugin.  It has no
-- dependency on a plugin manager or on nvim-lspconfig.
if vim.g.loaded_recite then
  return
end

vim.g.loaded_recite = true
-- init.lua is evaluated before runtimepath plugins.  Respect an explicit
-- setup call there instead of replacing it with the defaults below.
if not vim.g.recite_setup then
  local options = vim.g.recite_options
  require("recite").setup(type(options) == "table" and options or nil)
end
