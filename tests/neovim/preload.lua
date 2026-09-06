-- This is intentionally evaluated before runtimepath plugin scripts.  It
-- models both direct runtimepath use and plugin-manager preload ordering.
vim.opt.rtp:prepend(vim.env.RECITE_PLUGIN)
vim.opt.rtp:prepend(vim.env.RECITE_PARSER_ROOT)
vim.g.recite_options = {
  lsp = {
    cmd = { vim.env.RECITE_LSP },
  },
  commands = {
    binary = vim.env.RECITE_CLI,
  },
}
