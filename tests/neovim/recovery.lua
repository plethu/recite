local function fail(message)
  error("Neovim recovery check: " .. message, 0)
end

local function assert_true(value, message)
  if not value then
    fail(message)
  end
end

local function wait_for(predicate, message)
  assert_true(vim.wait(10000, predicate, 50), message)
end

local function escaped(path)
  return vim.fn.fnameescape(path)
end

local valid = vim.env.RECITE_TEST_PROJECT .. "/core_language_spike.recite"
vim.cmd("filetype on")
vim.cmd("edit " .. escaped(valid))
local recite = require("recite")
local clients
wait_for(function()
  clients = vim.lsp.get_clients({ bufnr = 0, name = "recite-lsp" })
  return #clients > 0 and clients[1].initialized
end, "recite-lsp did not attach before recovery checks")
local old_client_id = clients[1].id
local callback_seen = false
local callback_errors = { on_exit = false, on_init = false, restart_exhausted = false }
-- Capture the integration's user-facing reports while keeping the headless
-- smoke output focused on failures.
vim.notify = function(message, level)
  if type(message) == "string" then
    if message:find("Recite on_exit callback failed", 1, true) then
      callback_errors.on_exit = true
      callback_errors.on_exit_level = level
    elseif message:find("Recite on_init callback failed", 1, true) then
      callback_errors.on_init = true
      callback_errors.on_init_level = level
    elseif message:find("Recite language server restart attempts exhausted", 1, true) then
      callback_errors.restart_exhausted = true
      callback_errors.restart_exhausted_level = level
    end
  end
end
local capabilities = { workspace = { configuration = true } }
local init_options = { smoke = true }
recite.setup({
  lsp = {
    capabilities = capabilities,
    init_options = init_options,
    settings = { smoke = true },
    on_exit = function()
      callback_seen = true
      error("hostile on_exit callback")
    end,
  },
})
local restarted
wait_for(function()
  restarted = vim.lsp.get_clients({ bufnr = 0, name = "recite-lsp" })
  return #restarted > 0 and restarted[1].initialized and restarted[1].id ~= old_client_id
end, "material setup change did not restart the Recite client")
client = restarted[1]
assert_true(client.config.settings.smoke == true, "settings were not preserved across restart")
assert_true(client.config.init_options.smoke == true, "init_options were not preserved across restart")
assert_true(client.config.capabilities.workspace.configuration == true, "capabilities were not preserved across restart")

local client_id = client.id
assert_true(vim.lsp.get_client_by_id(client_id) ~= nil, "recite-lsp disappeared before shutdown")

local buffer_a = vim.fn.bufnr(valid)
wait_for(function()
  local candidates = vim.lsp.get_clients({ bufnr = buffer_a, name = "recite-lsp" })
  return #candidates > 0 and candidates[1].initialized
end, "the first project buffer lost its client")
local client_a = vim.lsp.get_clients({ bufnr = buffer_a, name = "recite-lsp" })[1]
vim.cmd("edit " .. escaped(vim.env.RECITE_SECOND_PROJECT .. "/core_language_spike.recite"))
local buffer_b = vim.api.nvim_get_current_buf()
wait_for(function()
  clients = vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })
  return #clients > 0 and clients[1].initialized
end, "second project did not receive a separate Recite client")
local client_b = clients[1]
assert_true(client_b.config.root_dir == vim.env.RECITE_SECOND_PROJECT, "second project root was not isolated")
assert_true(client_b.id ~= client_a.id, "separate project roots shared a client")

-- A same-name/root client started by a caller remains external ownership.
local external_id = vim.lsp.start({
  name = "recite-lsp",
  cmd = { vim.env.RECITE_LSP },
  root_dir = vim.env.RECITE_SECOND_PROJECT,
}, { bufnr = buffer_b, reuse_client = function() return false end })
wait_for(function()
  local external = vim.lsp.get_client_by_id(external_id)
  return external ~= nil and external.initialized
end, "external same-root probe did not initialize")
local reused_id = recite.start(buffer_b)
assert_true(reused_id == client_b.id, "Recite did not reuse its owned same-root client")
assert_true(vim.lsp.get_client_by_id(external_id) ~= nil, "external same-root client was adopted or stopped")
assert_true(recite.stop(external_id) == false, "Recite claimed ownership of an external client")
vim.lsp.get_client_by_id(external_id):stop(true)

-- Direct starts may override client material without changing global setup.
-- Repeated compatible starts reuse the existing owned client, while each
-- incompatible command/settings/init_options value receives its own client.
local function wait_initialized(client_id, label)
  local started
  wait_for(function()
    started = vim.lsp.get_client_by_id(client_id)
    return started ~= nil and started.initialized
  end, label .. " did not initialize")
  return started
end

local function stop_probe(client_id, label)
  assert_true(recite.stop(client_id), label .. " did not accept intentional stop")
  wait_for(function()
    return vim.lsp.get_client_by_id(client_id) == nil
  end, label .. " did not stop")
end

local default_override_id = recite.start(buffer_b)
assert_true(default_override_id == client_b.id,
  "a repeated default start did not reuse the owned client")
assert_true(recite.start(buffer_b) == default_override_id,
  "identical repeated starts did not reuse the owned client")

local settings_override_id = recite.start(buffer_b, {
  settings = { material_probe = { enabled = true } },
})
assert_true(settings_override_id ~= default_override_id,
  "a changed settings override silently reused the owned client")
local settings_override_client = wait_initialized(settings_override_id, "settings override client")
assert_true(settings_override_client.config.settings.material_probe.enabled == true,
  "the settings override was not applied to its distinct client")
assert_true(recite.start(buffer_b, {
  settings = { material_probe = { enabled = true } },
}) == settings_override_id, "an identical settings override did not reuse its client")
stop_probe(settings_override_id, "settings override client")

local command_override_id = recite.start(buffer_b, {
  cmd = { vim.env.RECITE_LSP, "--material-command-probe" },
})
assert_true(command_override_id ~= default_override_id,
  "a changed command override silently reused the owned client")
local command_override_client = wait_initialized(command_override_id, "command override client")
assert_true(command_override_client.config.cmd[2] == "--material-command-probe",
  "the command override was not applied to its distinct client")
assert_true(recite.start(buffer_b, {
  cmd = { vim.env.RECITE_LSP, "--material-command-probe" },
}) == command_override_id, "an identical command override did not reuse its client")
stop_probe(command_override_id, "command override client")

local init_options_override_id = recite.start(buffer_b, {
  init_options = { material_probe = { enabled = true } },
})
assert_true(init_options_override_id ~= default_override_id,
  "changed init_options silently reused the owned client")
local init_options_override_client = wait_initialized(init_options_override_id, "init_options override client")
assert_true(init_options_override_client.config.init_options.material_probe.enabled == true,
  "the init_options override was not applied to its distinct client")
assert_true(recite.start(buffer_b, {
  init_options = { material_probe = { enabled = true } },
}) == init_options_override_id, "an identical init_options override did not reuse its client")
stop_probe(init_options_override_id, "init_options override client")

-- Recovery must restart the exact direct-start variant that crashed. This
-- exercises every caller-owned material input that can affect its process or
-- protocol handshake, including a root override and callback identity.
local material_root = vim.fn.fnamemodify(vim.env.RECITE_SECOND_PROJECT, ":h")
local material_init_calls = 0
local material_exit_calls = 0
local material_attach_calls = 0
local material_overrides = {
  cmd = { vim.env.RECITE_LSP, "--material-recovery-probe" },
  root_dir = material_root,
  settings = { material_recovery = { enabled = true } },
  init_options = { material_recovery = { enabled = true } },
  capabilities = { workspace = { configuration = true } },
  on_attach = function()
    material_attach_calls = material_attach_calls + 1
  end,
  on_init = function()
    material_init_calls = material_init_calls + 1
  end,
  on_exit = function()
    material_exit_calls = material_exit_calls + 1
  end,
}
local material_id = recite.start(buffer_b, material_overrides)
assert_true(material_id ~= default_override_id,
  "material recovery variant unexpectedly reused the default client")
local material_client = wait_initialized(material_id, "material recovery client")
assert_true(material_client.config.root_dir == material_root,
  "material recovery variant lost its root override")
assert_true(material_client.config.cmd[2] == "--material-recovery-probe",
  "material recovery variant lost its command override")
assert_true(material_client.config.capabilities.workspace.configuration == true,
  "material recovery variant lost its capabilities")
assert_true(recite.start(buffer_b, material_overrides) == material_id,
  "an identical material recovery variant did not reuse its client")
material_client.rpc:terminate()
local recovered_material
wait_for(function()
  for _, candidate in ipairs(vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })) do
    if candidate.config.cmd[2] == "--material-recovery-probe"
      and candidate.id ~= material_id and candidate.initialized then
      recovered_material = candidate
      break
    end
  end
  return recovered_material ~= nil
end, "material recovery variant did not restart")
assert_true(recovered_material.config.root_dir == material_root,
  "recovered material variant lost its root override")
assert_true(recovered_material.config.settings.material_recovery.enabled == true,
  "recovered material variant lost its settings")
assert_true(recovered_material.config.init_options.material_recovery.enabled == true,
  "recovered material variant lost its init_options")
assert_true(recovered_material.config.capabilities.workspace.configuration == true,
  "recovered material variant lost its capabilities")
assert_true(material_init_calls >= 2 and material_exit_calls >= 1 and material_attach_calls >= 2,
  "material recovery did not preserve caller callbacks")
assert_true(recite.start(buffer_b, material_overrides) == recovered_material.id,
  "an identical material override did not reuse the recovered client")
stop_probe(recovered_material.id, "recovered material client")

-- A same-root variant must have an independent crash budget. Exhaust one
-- variant first, then start a second variant and prove it still gets a first
-- backoff recovery rather than inheriting the exhausted root's budget.
local budget_a = recite.start(buffer_b, { cmd = { vim.env.RECITE_LSP, "--budget-a" } })
wait_initialized(budget_a, "budget A client")
for attempt = 1, 4 do
  local crashed = vim.lsp.get_client_by_id(budget_a)
  assert_true(crashed ~= nil and crashed.rpc and crashed.rpc.terminate,
    "budget A crash probe is unavailable")
  crashed.rpc:terminate()
  if attempt < 4 then
    local replacement
    wait_for(function()
      for _, candidate in ipairs(vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })) do
        if candidate.config.cmd[2] == "--budget-a" and candidate.id ~= budget_a and candidate.initialized then
          replacement = candidate
          break
        end
      end
      return replacement ~= nil
    end, "budget A crash " .. attempt .. " did not recover")
    budget_a = replacement.id
  else
    wait_for(function()
      return vim.lsp.get_client_by_id(budget_a) == nil and callback_errors.restart_exhausted
    end, "budget A did not exhaust independently")
  end
end
local budget_b = recite.start(buffer_b, { cmd = { vim.env.RECITE_LSP, "--budget-b" } })
assert_true(budget_b ~= default_override_id,
  "same-root budget B variant reused an incompatible client")
local budget_b_client = wait_initialized(budget_b, "budget B client")
local budget_b_crash_time = (vim.uv or vim.loop).hrtime()
budget_b_client.rpc:terminate()
local budget_b_replacement
wait_for(function()
  for _, candidate in ipairs(vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })) do
    if candidate.config.cmd[2] == "--budget-b" and candidate.id ~= budget_b and candidate.initialized then
      budget_b_replacement = candidate
      break
    end
  end
  return budget_b_replacement ~= nil
end, "budget B inherited budget A's exhausted recovery")
assert_true(((vim.uv or vim.loop).hrtime() - budget_b_crash_time) / 1e6 < 500,
  "budget B did not recover using its independent first backoff")
stop_probe(budget_b_replacement.id, "budget B client")

-- A configured root_dir must be used when recovering every open buffer.
recite.setup({
  lsp = {
    root_dir = function(bufnr)
      return vim.fn.fnamemodify(vim.api.nvim_buf_get_name(bufnr), ":p:h")
    end,
  },
})
wait_for(function()
  local a = vim.lsp.get_clients({ bufnr = buffer_a, name = "recite-lsp" })
  local b = vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })
  return #a > 0 and a[1].initialized and #b > 0 and b[1].initialized
end, "configured root_dir did not reattach both project buffers")
client_a = vim.lsp.get_clients({ bufnr = buffer_a, name = "recite-lsp" })[1]
client_b = vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })[1]
local crashed_a, crashed_b = client_a.id, client_b.id
assert_true(client_a.rpc and client_a.rpc.terminate, "Neovim RPC crash probe is unavailable")
assert_true(client_b.rpc and client_b.rpc.terminate, "Neovim RPC crash probe is unavailable")
client_a.rpc:terminate()
client_b.rpc:terminate()
local restarted_a, restarted_b
wait_for(function()
  restarted_a = vim.lsp.get_clients({ bufnr = buffer_a, name = "recite-lsp" })
  restarted_b = vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })
  return #restarted_a > 0 and restarted_a[1].initialized and restarted_a[1].id ~= crashed_a
    and #restarted_b > 0 and restarted_b[1].initialized and restarted_b[1].id ~= crashed_b
end, "crash recovery did not reattach every open project buffer")
wait_for(function()
  return callback_seen and callback_errors.on_exit
end, "throwing on_exit callback was not observed without blocking crash recovery")
assert_true(callback_errors.on_exit_level == vim.log.levels.ERROR,
  "throwing on_exit callback was not reported at error level")

-- Wait past the stability window, then verify the next crash uses the first
-- backoff interval rather than accumulating an old retry count.
vim.wait(1300, function() return false end, 50)
local stable_client = restarted_b[1]
local stable_id = stable_client.id
local clock = vim.uv or vim.loop
local crash_time = clock.hrtime()
stable_client.rpc:terminate()
local stable_restart
wait_for(function()
  local candidates = vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })
  stable_restart = candidates[1]
  return stable_restart ~= nil and stable_restart.initialized and stable_restart.id ~= stable_id
end, "stable client did not recover after a second crash")
assert_true((clock.hrtime() - crash_time) / 1e6 < 500, "stable recovery retained an excessive backoff")

-- Reusing an active owned client must preserve the lifecycle's pending
-- stability timer. Without that, the timer is orphaned after the first crash
-- and this second crash incorrectly uses the slower backoff interval.
local reused_before_stable_id = stable_restart.id
assert_true(recite.start(buffer_b) == reused_before_stable_id,
  "Recite did not reuse the active owned client for the lifecycle probe")
vim.wait(1200, function() return false end, 50)
local reuse_crash_time = clock.hrtime()
stable_restart.rpc:terminate()
local reuse_restart
wait_for(function()
  local candidates = vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })
  reuse_restart = candidates[1]
  return reuse_restart ~= nil and reuse_restart.initialized and reuse_restart.id ~= reused_before_stable_id
end, "reused active client did not recover after a second crash")
local reuse_elapsed = (clock.hrtime() - reuse_crash_time) / 1e6
assert_true(reuse_elapsed < 180,
  "reusing an active client orphaned its stability timer and retained the second backoff")
stable_restart = reuse_restart

-- Disabling automatic startup while a crash is waiting for its first
-- backoff interval must invalidate the queued recovery.  The exit callback
-- proves the lifecycle has been removed before setup runs, which is the
-- hostile ordering that used to leave the timer owner unreachable.
callback_seen = false
local pending_disable_client = stable_restart
local pending_disable_crash_time = clock.hrtime()
pending_disable_client.rpc:terminate()
wait_for(function()
  return callback_seen
end, "pending-restart exit callback was not observed")
vim.wait(10, function() return false end, 5)
assert_true((clock.hrtime() - pending_disable_crash_time) / 1e6 < 100,
  "pending-restart probe did not reach setup during the initial backoff")
recite.setup({
  lsp = {
    autostart = false,
    root_dir = function(bufnr)
      return vim.fn.fnamemodify(vim.api.nvim_buf_get_name(bufnr), ":p:h")
    end,
  },
})
vim.wait(350, function() return false end, 50)
assert_true(#vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" }) == 0,
  "disabled autostart resurrected a client after pending crash recovery")

-- Rapid initialize-then-crash loops must consume the bounded recovery budget.
-- Initialization alone is not stability: the lifecycle only resets attempts
-- after its stability window, and the exhausted outcome is user-visible.
local rapid_init_calls = 0
recite.setup({
  lsp = {
    autostart = false,
    cmd = { vim.env.RECITE_LSP },
    root_dir = vim.env.RECITE_SECOND_PROJECT,
    on_init = function()
      rapid_init_calls = rapid_init_calls + 1
    end,
  },
})
wait_for(function()
  return #vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" }) == 0
end, "stable client did not stop before rapid recovery probe")
local rapid_id = recite.start(buffer_b)
assert_true(rapid_id ~= nil, "rapid recovery probe did not start")
for attempt = 1, 4 do
  local rapid_client
  wait_for(function()
    rapid_client = vim.lsp.get_client_by_id(rapid_id)
    return rapid_client ~= nil and rapid_client.initialized
  end, "rapid recovery client did not initialize before crash " .. attempt)
  assert_true(rapid_client.rpc and rapid_client.rpc.terminate,
    "Neovim RPC crash probe is unavailable for rapid client")
  rapid_client.rpc:terminate()
  if attempt < 4 then
    wait_for(function()
      local candidates = vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })
      rapid_client = candidates[1]
      return rapid_client ~= nil and rapid_client.initialized and rapid_client.id ~= rapid_id
    end, "rapid initialized crash " .. attempt .. " did not recover within bounded policy")
    rapid_id = rapid_client.id
  else
    wait_for(function()
      return #vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" }) == 0
        and callback_errors.restart_exhausted
    end, "rapid crash loop did not surface exhausted recovery without restarting")
  end
end
assert_true(rapid_init_calls == 4, "rapid crash loop reset its budget on initialize")
assert_true(callback_errors.restart_exhausted_level == vim.log.levels.ERROR,
  "exhausted recovery was not reported at error level")

-- A real server wrapper delays every server start by more than two seconds.
-- Each successful on_init must still reset the bounded restart
-- budget; otherwise the fourth post-init crash would be abandoned.
local delayed_init_calls = 0
recite.setup({
  lsp = {
    autostart = false,
    cmd = { vim.env.RECITE_DELAYED_LSP },
    root_dir = vim.env.RECITE_SECOND_PROJECT,
    on_init = function()
      delayed_init_calls = delayed_init_calls + 1
      error("hostile on_init callback")
    end,
  },
})
wait_for(function()
  return #vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" }) == 0
end, "previous owned clients did not stop before delayed wrapper probe")
local delayed_id = recite.start(buffer_b)
assert_true(delayed_id ~= nil, "delayed real-server wrapper did not start")
local delayed_client
wait_for(function()
  delayed_client = vim.lsp.get_client_by_id(delayed_id)
  return delayed_client ~= nil and delayed_client.initialized
end, "delayed real-server wrapper did not initialize")
wait_for(function()
  return delayed_init_calls == 1 and callback_errors.on_init
end, "throwing on_init callback was not observed after internal reset")
assert_true(callback_errors.on_init_level == vim.log.levels.ERROR,
  "throwing on_init callback was not reported at error level")

for attempt = 1, 4 do
  -- Initialization is not stability; let the real stability window elapse
  -- before each deliberate crash so this probe exercises budget reset.
  vim.wait(1100, function() return false end, 50)
  local crashed_id = delayed_id
  delayed_client = vim.lsp.get_client_by_id(crashed_id)
  assert_true(delayed_client ~= nil and delayed_client.initialized,
    "delayed client was not initialized before crash " .. attempt)
  assert_true(delayed_client.rpc and delayed_client.rpc.terminate,
    "Neovim RPC crash probe is unavailable for delayed client")
  delayed_client.rpc:terminate()
  wait_for(function()
    local candidates = vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" })
    delayed_client = candidates[1]
    return delayed_client ~= nil and delayed_client.initialized and delayed_client.id ~= crashed_id
  end, "delayed post-init crash " .. attempt .. " did not recover within bounded policy")
  delayed_id = delayed_client.id
end
assert_true(delayed_init_calls == 5, "delayed wrapper did not deliver every successful on_init event")
assert_true(recite.stop(delayed_id), "delayed owned client did not accept intentional stop")
wait_for(function()
  return #vim.lsp.get_clients({ bufnr = buffer_a, name = "recite-lsp" }) == 0
    and #vim.lsp.get_clients({ bufnr = buffer_b, name = "recite-lsp" }) == 0
end, "intentional shutdown did not stop all owned clients")
wait_for(function()
  return callback_seen
end, "caller on_exit was not preserved")

vim.cmd("qa!")
