local function fail(message)
  error("Neovim material check: " .. message, 0)
end

local function assert_true(value, message)
  if not value then
    fail(message)
  end
end

local function wait_for(predicate, message)
  assert_true(vim.wait(10000, predicate, 50), message)
end

local function wait_initialized(client_id, label)
  local client
  wait_for(function()
    client = vim.lsp.get_client_by_id(client_id)
    return client ~= nil and client.initialized
  end, label .. " did not initialize")
  return client
end

local function stop_probe(recite, client_id, label)
  assert_true(recite.stop(client_id), label .. " did not accept intentional stop")
  wait_for(function()
    return vim.lsp.get_client_by_id(client_id) == nil
  end, label .. " did not stop")
end

local function find_variant(argument, previous_id)
  for _, client in ipairs(vim.lsp.get_clients({ bufnr = 0, name = "recite-lsp" })) do
    if client.config.cmd[2] == argument and client.id ~= previous_id and client.initialized then
      return client
    end
  end
end

local valid = vim.env.RECITE_TEST_PROJECT .. "/core_language_spike.recite"
vim.cmd("filetype on")
vim.cmd("edit " .. vim.fn.fnameescape(valid))
local recite = require("recite")
local clients
wait_for(function()
  clients = vim.lsp.get_clients({ bufnr = 0, name = "recite-lsp" })
  return #clients > 0 and clients[1].initialized
end, "the default Recite client did not initialize")
local default_id = clients[1].id
assert_true(recite.start() == default_id, "an identical default start did not reuse its client")

local settings_id = recite.start(0, { settings = { material_probe = { enabled = true } } })
assert_true(settings_id ~= default_id, "changed settings silently reused the default client")
local settings_client = wait_initialized(settings_id, "settings override client")
assert_true(settings_client.config.settings.material_probe.enabled == true,
  "settings override was not applied")
assert_true(recite.start(0, { settings = { material_probe = { enabled = true } } }) == settings_id,
  "identical settings override did not reuse its client")
stop_probe(recite, settings_id, "settings override client")

local command_id = recite.start(0, { cmd = { vim.env.RECITE_LSP, "--material-command" } })
assert_true(command_id ~= default_id, "changed command silently reused the default client")
local command_client = wait_initialized(command_id, "command override client")
assert_true(command_client.config.cmd[2] == "--material-command", "command override was not applied")
assert_true(recite.start(0, { cmd = { vim.env.RECITE_LSP, "--material-command" } }) == command_id,
  "identical command override did not reuse its client")
stop_probe(recite, command_id, "command override client")

local init_id = recite.start(0, { init_options = { material_probe = { enabled = true } } })
assert_true(init_id ~= default_id, "changed init_options silently reused the default client")
local init_client = wait_initialized(init_id, "init_options override client")
assert_true(init_client.config.init_options.material_probe.enabled == true,
  "init_options override was not applied")
assert_true(recite.start(0, { init_options = { material_probe = { enabled = true } } }) == init_id,
  "identical init_options override did not reuse its client")
stop_probe(recite, init_id, "init_options override client")

local material_root = vim.fn.fnamemodify(vim.env.RECITE_TEST_PROJECT, ":h")
local init_calls, exit_calls, attach_calls = 0, 0, 0
local material_overrides = {
  cmd = { vim.env.RECITE_LSP, "--material-recovery" },
  root_dir = material_root,
  settings = { material_recovery = { enabled = true } },
  init_options = { material_recovery = { enabled = true } },
  capabilities = { workspace = { configuration = true } },
  on_attach = function() attach_calls = attach_calls + 1 end,
  on_init = function() init_calls = init_calls + 1 end,
  on_exit = function() exit_calls = exit_calls + 1 end,
}
local material_id = recite.start(0, material_overrides)
assert_true(material_id ~= default_id, "material variant unexpectedly reused the default client")
local material_client = wait_initialized(material_id, "material recovery client")
assert_true(material_client.config.root_dir == material_root, "material root override was lost")
assert_true(recite.start(0, material_overrides) == material_id,
  "identical material override did not reuse its client")
material_client.rpc:terminate()
local recovered
wait_for(function()
  recovered = find_variant("--material-recovery", material_id)
  return recovered ~= nil
end, "material variant did not recover")
assert_true(recovered.config.root_dir == material_root, "recovered root override was lost")
assert_true(recovered.config.settings.material_recovery.enabled == true, "recovered settings were lost")
assert_true(recovered.config.init_options.material_recovery.enabled == true, "recovered init_options were lost")
assert_true(recovered.config.capabilities.workspace.configuration == true, "recovered capabilities were lost")
assert_true(init_calls >= 2 and exit_calls >= 1 and attach_calls >= 2,
  "recovered callbacks were not preserved")
assert_true(recite.start(0, material_overrides) == recovered.id,
  "identical material override did not reuse the recovered client")
stop_probe(recite, recovered.id, "recovered material client")

local exhausted = false
vim.notify = function(message)
  if type(message) == "string" and message:find("restart attempts exhausted", 1, true) then
    exhausted = true
  end
end
local budget_id = recite.start(0, { cmd = { vim.env.RECITE_LSP, "--budget-a" } })
wait_initialized(budget_id, "budget A client")
for attempt = 1, 4 do
  local crashed = vim.lsp.get_client_by_id(budget_id)
  assert_true(crashed ~= nil and crashed.rpc and crashed.rpc.terminate, "budget A crash probe unavailable")
  crashed.rpc:terminate()
  if attempt < 4 then
    local replacement
    wait_for(function()
      replacement = find_variant("--budget-a", budget_id)
      return replacement ~= nil
    end, "budget A crash did not recover")
    budget_id = replacement.id
  else
    wait_for(function() return vim.lsp.get_client_by_id(budget_id) == nil and exhausted end,
      "budget A did not exhaust independently")
  end
end
local other_id = recite.start(0, { cmd = { vim.env.RECITE_LSP, "--budget-b" } })
assert_true(other_id ~= default_id, "budget B reused an incompatible client")
local other = wait_initialized(other_id, "budget B client")
local crash_time = (vim.uv or vim.loop).hrtime()
other.rpc:terminate()
local other_replacement
wait_for(function()
  other_replacement = find_variant("--budget-b", other_id)
  return other_replacement ~= nil
end, "budget B inherited budget A's exhausted recovery")
assert_true(((vim.uv or vim.loop).hrtime() - crash_time) / 1e6 < 500,
  "budget B did not use its independent first backoff")
stop_probe(recite, other_replacement.id, "budget B client")

vim.cmd("qa!")
