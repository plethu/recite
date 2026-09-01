-- Filetype registration is also performed by lua/recite.lua.  Keeping this
-- tiny ftdetect entrypoint makes the file association work with Neovim's
-- standard runtimepath loading even when no plugin manager is present.
vim.filetype.add({ extension = { recite = "recite" } })
