-- Input and buffer fencing helpers for the structured command adapter.
-- These helpers deliberately know about Neovim buffers, but not CLI record
-- shapes or process lifecycle.
local protocol = require("recite.command_protocol")
local M = {}

function M.absolute(path)
  local absolute = vim.fn.fnamemodify(path, ":p")
  if vim.fs and vim.fs.normalize then absolute = vim.fs.normalize(absolute) end
  if absolute ~= "/" then absolute = absolute:gsub("/+$", "") end
  return absolute
end

function M.inside(path, root)
  path, root = M.absolute(path), M.absolute(root)
  if root == "/" then return path:sub(1, 1) == "/" end
  return path == root or path:sub(1, #root + 1) == root .. "/"
end

function M.project_root(options, root_dir)
  if options and options.project_root then return M.absolute(options.project_root) end
  if options and options.path and vim.fn.isdirectory(options.path) ~= 1 then return M.absolute(vim.fn.fnamemodify(options.path, ":h")) end
  return M.absolute(type(root_dir) == "function" and root_dir(0) or root_dir)
end

function M.clean_snapshot(root)
  local bufnr = vim.api.nvim_get_current_buf()
  if vim.bo[bufnr].filetype ~= "recite" then return nil end
  local name = vim.api.nvim_buf_get_name(bufnr)
  if name == "" or not M.inside(name, root) then return nil end
  if vim.bo[bufnr].modified then return nil, protocol.error("document_unsaved") end
  return { bufnr = bufnr, path = M.absolute(name), changedtick = vim.api.nvim_buf_get_changedtick(bufnr), root = root }
end

function M.snapshot_current(snapshot)
  if not snapshot or not vim.api.nvim_buf_is_valid(snapshot.bufnr) then return false end
  return not vim.bo[snapshot.bufnr].modified and vim.api.nvim_buf_get_changedtick(snapshot.bufnr) == snapshot.changedtick
    and M.absolute(vim.api.nvim_buf_get_name(snapshot.bufnr)) == snapshot.path
end

function M.command_binary(config)
  local binary = config.binary or vim.g.recite_binary or "recite"
  if type(binary) == "table" then binary = binary[1] end
  if type(binary) ~= "string" or binary == "" or vim.fn.executable(binary) ~= 1 then return nil, binary end
  return binary
end

function M.paths_for(options, root)
  if options and options.paths then
    local values = type(options.paths) == "table" and options.paths or { options.paths }
    local result = {}
    if #values == 0 then return nil end
    for _, path in ipairs(values) do
      if type(path) ~= "string" or path == "" then return nil end
      path = M.absolute(path)
      if not M.inside(path, root) then return nil end
      result[#result + 1] = path
    end
    return result
  end
  return { root }
end

return M
