import { validDiagnosticRecord } from "./command-diagnostics.js";
import { integerInRange } from "./command-protocol.js";
import {
  checkReasons,
  engineReasons,
  errorCategories,
  errorCodes,
  failures,
  freshnesses,
  notAttemptedReasons,
  operations,
  outcomes,
  publications,
  publishFailureReasons,
  recoveryIoKinds,
  recoveryReasons,
  refusalReasons,
  staleReasons,
  statuses,
} from "./watch-record-tags.js";

export function validBuildStart(value) {
  return Boolean(
    value && integerInRange(value.generation, "0", "18446744073709551615")
      && (value.trigger === "initial" || value.trigger === "input_changed"),
  );
}

export function validBuildCompleted(value) {
  if (
    !value || !integerInRange(value.generation, "0", "18446744073709551615")
    || (value.snapshot_generation !== null
      && !integerInRange(value.snapshot_generation, "0", "18446744073709551615"))
    || !statuses.has(value.status) || !validOutcome(value.outcome)
    || !sortedStrings(value.inputs) || !Array.isArray(value.diagnostics)
    || !value.diagnostics.every(validDiagnosticRecord) || !Array.isArray(value.artifacts)
    || !value.artifacts.every(validArtifact) || !validFreshness(value.freshness)
    || !validPublication(value.publication) || !Array.isArray(value.recovery)
    || !value.recovery.every(validRecovery) || !validRestartGuidance(value.restart_guidance)
    || (value.cancellation !== undefined && !validCancellation(value.cancellation))
    || (value.failure !== undefined && !validFailure(value.failure))
    || (value.error !== undefined && !validStructuredError(value.error))
  ) return false;
  const expected = {
    succeeded: new Set(["fresh", "recovery_required"]),
    failed: new Set([
      "diagnostics",
      "recovery_required",
      "freshness_failure",
      "operational_failure",
      "publication_failure",
    ]),
    stale: new Set(["stale"]),
    cancelled: new Set(["cancelled"]),
    superseded: new Set(["superseded"]),
    unknown: new Set(["unknown"]),
  };
  if (!expected[value.status]?.has(value.outcome.type)) return false;
  if (value.status === "cancelled" && value.cancellation?.type !== "user") return false;
  if (value.status === "superseded" && value.cancellation?.type !== "superseded") return false;
  if (value.outcome.type === "diagnostics" && value.failure?.type !== "diagnostics") return false;
  return true;
}

export function validStop(value) {
  return Boolean(
    value && validTagged(value.reason) && keys(value.reason, ["type"])
      && ((value.reason.type === "fatal" && validStructuredError(value.error))
        || (value.reason.type === "cancelled" && value.error === undefined)),
  );
}

export function validCancellation(value) {
  if (!value || typeof value !== "object" || typeof value.type !== "string") return false;
  if (value.type === "user" || value.type === "unknown") return keys(value, ["type"]);
  return value.type === "superseded" && keys(value, ["type", "by_generation"])
    && integerInRange(value.by_generation, "0", "18446744073709551615");
}

function validFreshness(value) {
  return Boolean(
    value && freshnesses.has(value.type)
      && (value.type === "stale"
        ? keys(value, ["type", "reasons"]) && sortedTaggedStrings(value.reasons, staleReasons)
        : keys(value, ["type"])),
  );
}

function validOutcome(value) {
  return Boolean(value && outcomes.has(value.type) && keys(value, ["type"]));
}

function validPublication(value) {
  if (!value || typeof value !== "object" || !publications.has(value.type)) return false;
  switch (value.type) {
    case "not_attempted":
      return keys(value, ["type", "reason"]) && notAttemptedReasons.has(value.reason);
    case "published":
      return keys(value, ["type", "targets"]) && sortedStrings(value.targets);
    case "partial":
      return keys(value, ["type", "committed", "failed", "remaining", "recovery"])
        && sortedStrings(value.committed) && string(value.failed)
        && sortedStrings(value.remaining) && sortedStrings(value.recovery);
    case "indeterminate":
      return keys(value, ["type", "attempted", "recovery"]) && sortedStrings(value.attempted)
        && sortedStrings(value.recovery);
    case "refused":
      return keys(value, ["type", "reason"]) && refusalReasons.has(value.reason);
    case "unknown":
      return keys(value, ["type"]);
    default:
      return false;
  }
}

function validRecovery(value) {
  return Boolean(
    value && keys(value, ["marker", "reason"], value.detail === undefined ? [] : ["detail"])
      && machinePath(value.marker) && recoveryReasons.has(value.reason)
      && (value.detail === undefined || keys(value.detail, ["type", "kind", "raw_os_error"])
          && value.detail.type === "io" && recoveryIoKinds.has(value.detail.kind)
          && (value.detail.raw_os_error === null
            || Number.isSafeInteger(value.detail.raw_os_error))),
  );
}

function validRestartGuidance(value) {
  return Boolean(
    value && keys(value, ["type", "decision"]) && value.type === "host_policy_required"
      && value.decision === "unspecified",
  );
}

export function validStructuredError(value) {
  return Boolean(
    value && typeof value === "object" && errorCategories.has(value.category)
      && errorCodes.has(value.code) && operations.has(value.operation)
      && Object.keys(value).every((key) =>
        ["category", "code", "operation", "path", "related_path", "details"].includes(key)
      )
      && (value.path === undefined || machinePath(value.path))
      && (value.related_path === undefined || machinePath(value.related_path))
      && (value.details === undefined || validErrorDetails(value.details)),
  );
}

function validErrorDetails(value) {
  if (!value || typeof value !== "object" || typeof value.type !== "string") return false;
  switch (value.type) {
    case "fixture_choice":
      return keys(value, ["type", "choice", "prompt_keys"]) && string(value.choice)
        && uniqueStrings(value.prompt_keys);
    case "fixture_choice_index":
      return keys(value, ["type", "index", "choice_count", "prompt_keys"])
        && integerInRange(value.index, "0", "18446744073709551615")
        && integerInRange(value.choice_count, "0", "18446744073709551615")
        && uniqueStrings(value.prompt_keys);
    case "ambiguous_fixture":
      return keys(value, ["type", "block", "prompt_count"]) && string(value.block)
        && integerInRange(value.prompt_count, "0", "18446744073709551615");
    case "missing_fixture_choice":
      return keys(value, ["type", "prompt_keys"]) && uniqueStrings(value.prompt_keys);
    case "blocking_effect":
      return keys(value, ["type", "effect"]) && string(value.effect);
    case "locale":
      return keys(value, ["type", "field", "locale"]) && string(value.field)
        && string(value.locale);
    case "catalog_spec":
      return keys(value, ["type", "spec"]) && string(value.spec);
    case "watch":
      return keys(value, ["type", "kind"]) && string(value.kind);
    case "watch_target":
      return keys(value, ["type", "kind", "target"]) && string(value.kind) && string(value.target);
    default:
      return false;
  }
}

function validFailure(value) {
  if (!value || typeof value !== "object" || !failures.has(value.type)) return false;
  switch (value.type) {
    case "check":
      return keys(value, ["type", "reason"]) && checkReasons.has(value.reason);
    case "engine":
      return keys(value, ["type", "reason"]) && engineReasons.has(value.reason);
    case "duplicate_target":
      return keys(value, ["type", "target"]) && string(value.target);
    case "preparation":
      return keys(value, ["type", "target", "reason"]) && string(value.target)
        && publishFailureReasons.has(value.reason);
    case "diagnostics":
    case "invalid_publication":
    case "freshness":
    case "unknown":
      return keys(value, ["type"]);
    default:
      return false;
  }
}

function validArtifact(value) {
  return Boolean(
    value && keys(value, ["path", "size_bytes"]) && machinePath(value.path)
      && integerInRange(value.size_bytes, "0", "18446744073709551615"),
  );
}

function machinePath(value) {
  return Boolean(
    value && typeof value === "object" && keys(value, ["encoding", "value"])
      && ((value.encoding === "utf8" && typeof value.value === "string")
        || (value.encoding === "unix_bytes" && typeof value.value === "string"
          && /^[0-9a-f]*$/u.test(value.value) && value.value.length % 2 === 0)
        || (value.encoding === "windows_wtf16" && Array.isArray(value.value)
          && value.value.every((unit) =>
            Number.isSafeInteger(unit) && unit >= 0 && unit <= 0xffff
          ))),
  );
}

function validTagged(value) {
  return Boolean(value && typeof value === "object" && typeof value.type === "string");
}
function string(value) {
  return typeof value === "string" && value.length > 0;
}
function sortedStrings(value) {
  return Array.isArray(value) && value.every((item) => typeof item === "string")
    && new Set(value).size === value.length
    && value.every((item, index) => index === 0 || compareUtf8(value[index - 1], item) < 0);
}
function uniqueStrings(value) {
  return Array.isArray(value) && value.every((item) => typeof item === "string")
    && new Set(value).size === value.length;
}
function sortedTaggedStrings(value, allowed) {
  return Array.isArray(value) && value.every((item) =>
    item && typeof item === "object"
    && allowed.has(item.type) && keys(item, ["type"])
  )
    && new Set(value.map((item) => item.type)).size === value.length
    && value.every((item, index) =>
      index === 0 || compareUtf8(value[index - 1].type, item.type) < 0
    );
}

function compareUtf8(left, right) {
  return Buffer.compare(Buffer.from(left), Buffer.from(right));
}
function keys(value, expected) {
  return JSON.stringify(Object.keys(value).sort()) === JSON.stringify(expected.slice().sort());
}
