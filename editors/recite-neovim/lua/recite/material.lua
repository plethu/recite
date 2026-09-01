local M = {}

local keys = {
  "name", "root_dir", "root_dir_spec", "root_markers", "cmd", "settings",
  "init_options", "capabilities", "on_attach", "on_init", "on_exit",
}

local function same_value(left, right)
  if type(left) == "function" or type(right) == "function" then
    return left == right
  end
  return vim.deep_equal(left, right)
end

function M.same(client, config)
  local actual = client.config.recite_material
  local expected = config.recite_material
  if not actual or not expected then
    return false
  end
  for _, key in ipairs(keys) do
    if not same_value(actual[key], expected[key]) then
      return false
    end
  end
  return true
end

function M.root_config(lifecycle, default_lsp)
  if not lifecycle.material then
    return default_lsp
  end
  return {
    root_dir = lifecycle.material.root_dir_spec,
    root_markers = lifecycle.material.root_markers,
  }
end

function M.restart_overrides(material)
  local overrides = {
    name = material.name,
    cmd = vim.deepcopy(material.cmd),
    root_markers = vim.deepcopy(material.root_markers),
    settings = vim.deepcopy(material.settings),
    init_options = vim.deepcopy(material.init_options),
    capabilities = vim.deepcopy(material.capabilities),
    on_attach = material.on_attach,
    on_init = material.on_init,
    on_exit = material.on_exit,
  }
  if material.root_dir_spec ~= nil then
    overrides.root_dir = material.root_dir_spec
  end
  return overrides
end

return M
