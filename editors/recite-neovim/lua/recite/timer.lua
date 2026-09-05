-- Own libuv timers directly.  `vim.defer_fn` has changed its return shape
-- across supported Neovim versions and its handle is not a Vimscript timer
-- ID; using `vim.fn.timer_stop` for it can leak or raise Lua errors.
local M = {}

function M.stop(handle)
  if not handle then return end
  if type(handle) ~= "userdata" and type(handle) ~= "table" then return end
  local uv = vim.uv or vim.loop
  if not uv then return end
  pcall(uv.timer_stop, handle)
  if handle.close then
    local closing = handle.is_closing and handle:is_closing()
    if not closing then pcall(handle.close, handle) end
  end
end

function M.after(milliseconds, callback)
  local uv = vim.uv or vim.loop
  assert(uv and uv.new_timer, "Neovim libuv timers are unavailable")
  local handle = uv.new_timer()
  handle:start(milliseconds, 0, function()
    M.stop(handle)
    vim.schedule(callback)
  end)
  return handle
end

return M
