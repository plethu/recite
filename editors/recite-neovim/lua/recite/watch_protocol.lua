local protocol = require("recite.command_protocol")
local diagnostic = require("recite.diagnostic_protocol")

local M = {}

local CONTROL_ERRORS = { malformed = true, unsupported_version = true, unsupported_command = true,
  unsupported_action = true, invocation_mismatch = true }
local STATUSES = { succeeded = true, failed = true, stale = true, cancelled = true, superseded = true, unknown = true }
local OUTCOMES = { fresh = true, diagnostics = true, stale = true, recovery_required = true,
  freshness_failure = true, operational_failure = true, publication_failure = true, unknown = true,
  cancelled = true, superseded = true }
local EXPECTED_OUTCOMES = {
  succeeded = { fresh = true, recovery_required = true },
  failed = { diagnostics = true, recovery_required = true, freshness_failure = true, operational_failure = true, publication_failure = true },
  stale = { stale = true }, cancelled = { cancelled = true }, superseded = { superseded = true }, unknown = { unknown = true },
}
local FRESHNESS_REASONS = { build_generation = true, snapshot_generation = true, fingerprints = true, unknown = true }
local NOT_ATTEMPTED_REASONS = { build_failed = true, cancelled = true, superseded = true, stale = true, no_candidates = true, preparation_failed = true, invalid_outcome = true, unknown = true }
local REFUSAL_REASONS = { stale_build_generation = true, stale_snapshot_generation = true, stale_fingerprints = true, request_identity_mismatch = true, unknown = true }
local RECOVERY_REASONS = { stage_cleanup_failed = true, publication_indeterminate = true, publication_uncommitted = true }
local FAILURE_TYPES = { check = true, diagnostics = true, engine = true, duplicate_target = true, preparation = true, invalid_publication = true, freshness = true, unknown = true }

local function valid_tag(value)
  return protocol.object(value) and type(value.type) == "string"
end

local function sorted_strings(value)
  if not protocol.array(value) then return false end
  local previous
  local seen = {}
  for _, item in ipairs(value) do
    if type(item) ~= "string" or seen[item] or previous and item <= previous then return false end
    seen[item] = true
    previous = item
  end
  return true
end

local function valid_artifact(value)
  return protocol.exact(value, { "path", "size_bytes" }) and protocol.machine_path(value.path)
    and protocol.integer_in_range(value.size_bytes, "0", "18446744073709551615")
end

local function valid_outcome(value)
  return protocol.exact(value, { "type" }) and OUTCOMES[value.type]
end

local function valid_freshness(value)
  if not valid_tag(value) then return false end
  if value.type == "stale" then
    return protocol.exact(value, { "type", "reasons" }) and protocol.array(value.reasons) and protocol.every(value.reasons, function(item)
      return valid_tag(item) and protocol.exact(item, { "type" }) and FRESHNESS_REASONS[item.type]
    end)
  end
  return protocol.exact(value, { "type" }) and (value.type == "fresh" or value.type == "unknown")
end

local function valid_publication(value)
  if not valid_tag(value) then return false end
  if value.type == "not_attempted" then return protocol.exact(value, { "type", "reason" }) and NOT_ATTEMPTED_REASONS[value.reason] end
  if value.type == "published" then return protocol.exact(value, { "type", "targets" }) and sorted_strings(value.targets) end
  if value.type == "partial" then return protocol.exact(value, { "type", "committed", "failed", "remaining", "recovery" })
    and sorted_strings(value.committed) and type(value.failed) == "string" and sorted_strings(value.remaining) and sorted_strings(value.recovery) end
  if value.type == "indeterminate" then return protocol.exact(value, { "type", "attempted", "recovery" }) and sorted_strings(value.attempted) and sorted_strings(value.recovery) end
  if value.type == "refused" then return protocol.exact(value, { "type", "reason" }) and REFUSAL_REASONS[value.reason] end
  return value.type == "unknown" and protocol.exact(value, { "type" })
end

local function valid_recovery(value)
  return protocol.keys(value, { "marker", "reason" }, { "detail" }) and protocol.machine_path(value.marker) and RECOVERY_REASONS[value.reason]
    and (value.detail == nil or protocol.exact(value.detail, { "type", "kind", "raw_os_error" }) and value.detail.type == "io"
      and vim.tbl_contains({ "already_exists", "invalid_input", "not_found", "permission_denied", "other" }, value.detail.kind)
      and (value.detail.raw_os_error == vim.NIL or value.detail.raw_os_error == nil or protocol.integer_in_range(value.detail.raw_os_error, "-2147483648", "2147483647")))
end

local function valid_cancellation(value)
  if not valid_tag(value) then return false end
  if value.type == "user" or value.type == "unknown" then return protocol.exact(value, { "type" }) end
  return value.type == "superseded" and protocol.exact(value, { "type", "by_generation" })
    and protocol.integer_in_range(value.by_generation, "0", "18446744073709551615")
end

local function valid_failure(value)
  if not valid_tag(value) or not FAILURE_TYPES[value.type] then return false end
  if value.type == "check" then return protocol.exact(value, { "type", "reason" }) and vim.tbl_contains({ "request_mismatch", "freshness_mismatch", "unknown" }, value.reason) end
  if value.type == "engine" then return protocol.exact(value, { "type", "reason" }) and vim.tbl_contains({ "invalid_output", "host", "unknown" }, value.reason) end
  if value.type == "duplicate_target" then return protocol.exact(value, { "type", "target" }) and type(value.target) == "string" and value.target ~= "" end
  if value.type == "preparation" then return protocol.exact(value, { "type", "target", "reason" }) and type(value.target) == "string" and value.target ~= "" and vim.tbl_contains({ "rejected", "storage", "unknown" }, value.reason) end
  return protocol.exact(value, { "type" })
end

local function valid_completed(value)
  if not protocol.keys(value, { "generation", "snapshot_generation", "status", "outcome", "inputs", "diagnostics", "artifacts", "freshness", "publication", "recovery", "restart_guidance" }, { "cancellation", "failure", "error" })
    or not protocol.integer_in_range(value.generation, "0", "18446744073709551615")
    or not (value.snapshot_generation == nil or value.snapshot_generation == vim.NIL or protocol.integer_in_range(value.snapshot_generation, "0", "18446744073709551615"))
    or not STATUSES[value.status] or not valid_outcome(value.outcome) or not sorted_strings(value.inputs)
    or not protocol.array(value.diagnostics) or not protocol.array(value.artifacts) or not protocol.array(value.recovery)
    or not valid_freshness(value.freshness) or not valid_publication(value.publication)
    or not protocol.exact(value.restart_guidance, { "type", "decision" })
    or value.restart_guidance.type ~= "host_policy_required" or value.restart_guidance.decision ~= "unspecified" then return false end
  for _, diagnostic_record in ipairs(value.diagnostics) do if not diagnostic.valid(diagnostic_record) then return false end end
  for _, artifact in ipairs(value.artifacts) do if not valid_artifact(artifact) then return false end end
  for _, recovery in ipairs(value.recovery) do if not valid_recovery(recovery) then return false end end
  if value.cancellation ~= nil and not valid_cancellation(value.cancellation) then return false end
  if value.failure ~= nil and not valid_failure(value.failure) then return false end
  if value.error ~= nil and not protocol.valid_error(value.error) then return false end
  if value.status == "cancelled" and (not value.cancellation or value.cancellation.type ~= "user") then return false end
  if value.status == "superseded" and (not value.cancellation or value.cancellation.type ~= "superseded") then return false end
  if not EXPECTED_OUTCOMES[value.status] or not EXPECTED_OUTCOMES[value.status][value.outcome.type] then return false end
  if value.outcome.type == "diagnostics" and (not value.failure or value.failure.type ~= "diagnostics") then return false end
  return true
end

local Validator = {}
Validator.__index = Validator

function M.new(invocation_id, expected_root)
  return setmetatable({ command = "watch", invocation_id = invocation_id, expected_root = expected_root,
    sequence = protocol.integer("0"), started = false, phase = "before_started", active_generation = nil,
    last_generation = nil, cancel_requested = false, stopped = false, stop_reason = nil, project_root = nil }, Validator)
end

function Validator:consume(record)
  if self.stopped then error(protocol.error("records_after_stopped")) end
  if not protocol.keys(record, { "version", "sequence", "event", "command", "data" }, { "invocation_id" }) then error(protocol.error("invalid_envelope")) end
  protocol.validate_envelope(record, self.command, self.invocation_id, self.sequence)
  self.sequence = protocol.integer(protocol.increment_integer(self.sequence))
  if record.event == "watch.started" then self:started_record(record)
  elseif record.event == "watch.build.started" then self:build_started(record)
  elseif record.event == "watch.build.completed" then self:build_completed(record)
  elseif record.event == "watch.waiting" then
    if not self.started or self.phase ~= "awaiting_wait" or not protocol.exact(record.data, {}) then error(protocol.error("invalid_watch_waiting")) end
    self.phase = "awaiting_build"
  elseif record.event == "watch.cancel.requested" then self:cancel(record)
  elseif record.event == "watch.control.error" then
    if not self.started or not protocol.exact(record.data, { "error" }) or not protocol.exact(record.data.error, { "type" }) or not CONTROL_ERRORS[record.data.error.type] then error(protocol.error("invalid_control_error")) end
  elseif record.event == "watch.notify.error" then
    if not self.started or not protocol.exact(record.data, { "error" }) or not protocol.exact(record.data.error, { "type" }) or record.data.error.type ~= "watcher" then error(protocol.error("invalid_notify_error")) end
  elseif record.event == "watch.stopped" then self:stopped_record(record)
  else error(protocol.error("unknown_watch_event")) end
  return record
end

function Validator:started_record(record)
  if self.started or not protocol.exact(record.data, { "project_root" }) or not protocol.machine_path(record.data.project_root) then error(protocol.error("invalid_watch_started")) end
  local value = protocol.machine_path_value(record.data.project_root)
  if not value then error(protocol.error("invalid_watch_project_root")) end
  local root = vim.fn.fnamemodify(value, ":p")
  if vim.fs and vim.fs.normalize then root = vim.fs.normalize(root) end
  if root:sub(1, 1) ~= "/" then error(protocol.error("invalid_watch_project_root")) end
  if self.expected_root then
    local expected = vim.fn.fnamemodify(self.expected_root, ":p")
    if vim.fs and vim.fs.normalize then expected = vim.fs.normalize(expected) end
    if vim.fn.resolve(root) ~= vim.fn.resolve(expected) then error(protocol.error("watch_project_root_mismatch")) end
  end
  self.project_root = root
  self.started = true
  self.phase = "awaiting_build"
end

function Validator:build_started(record)
  local expected = self.last_generation == nil and "initial" or "input_changed"
  if not self.started or self.cancel_requested or self.phase ~= "awaiting_build" or not protocol.exact(record.data, { "generation", "trigger" })
    or not protocol.integer_in_range(record.data.generation, "0", "18446744073709551615") or record.data.trigger ~= expected
    or self.last_generation == nil and protocol.compare_integer(record.data.generation, "0") ~= 0
    or self.last_generation ~= nil and protocol.compare_integer(record.data.generation, self.last_generation) <= 0 then error(protocol.error("invalid_build_started")) end
  self.active_generation = record.data.generation
  self.last_generation = record.data.generation
  self.phase = "building"
end

function Validator:build_completed(record)
  if not self.started or self.phase ~= "building" or self.active_generation == nil or not valid_completed(record.data)
    or protocol.compare_integer(record.data.generation, self.active_generation) ~= 0 then error(protocol.error("invalid_build_completed")) end
  self.active_generation = nil
  self.phase = self.cancel_requested and "stopped_ready" or "awaiting_wait"
end

function Validator:cancel(record)
  if not self.started or self.cancel_requested or not protocol.exact(record.data, { "cancellation" }) or not valid_cancellation(record.data.cancellation)
    or record.data.cancellation.type ~= "user" or self.stop_reason or (self.phase ~= "awaiting_build" and self.phase ~= "building") then error(protocol.error("invalid_cancel")) end
  self.cancel_requested = true
  if self.phase == "awaiting_build" then self.phase = "stopped_ready" end
end

function Validator:stopped_record(record)
  local fatal_after_completed = self.phase == "awaiting_wait"
  if not self.started or self.stop_reason or not protocol.keys(record.data, { "reason" }, { "error" }) or not valid_tag(record.data.reason)
    or (record.data.reason.type ~= "fatal" and record.data.reason.type ~= "cancelled")
    or record.data.reason.type == "fatal" and not protocol.valid_error(record.data.error)
    or record.data.reason.type == "cancelled" and record.data.error ~= nil
    or (self.phase ~= "awaiting_build" and self.phase ~= "stopped_ready" and not fatal_after_completed) then error(protocol.error("invalid_watch_stopped")) end
  if record.data.reason.type == "cancelled" and not self.cancel_requested then error(protocol.error("invalid_watch_stopped")) end
  self.stop_reason = record.data.reason.type
  self.stopped = true
  self.phase = "stopped"
end

function Validator:finish(exit_code)
  if not self.stopped or self.active_generation ~= nil or type(exit_code) ~= "number" then error(protocol.error("watch_not_stopped")) end
  local expected = self.stop_reason == "cancelled" and 0 or 1
  if exit_code ~= expected then error(protocol.error("watch_exit_mismatch")) end
end

M.Validator = Validator
M.valid_completed = valid_completed
M.valid_cancellation = valid_cancellation

return M
