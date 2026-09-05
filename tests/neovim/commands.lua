-- The command checks are split by ownership so protocol failures cannot be
-- hidden by the process/E2E lane.  Keep this entry point for the gate and for
-- contributors running the command slice directly.
dofile(vim.env.RECITE_REPO_ROOT .. "/tests/neovim/commands_protocol.lua")
dofile(vim.env.RECITE_REPO_ROOT .. "/tests/neovim/commands_lifecycle.lua")
vim.cmd("qa!")
