// Closed v1 tags for the structured watch protocol.
export const statuses = new Set([
  "succeeded",
  "failed",
  "stale",
  "cancelled",
  "superseded",
  "unknown",
]);
export const outcomes = new Set([
  "fresh",
  "diagnostics",
  "stale",
  "recovery_required",
  "freshness_failure",
  "operational_failure",
  "publication_failure",
  "unknown",
  "cancelled",
  "superseded",
]);
export const freshnesses = new Set(["fresh", "stale", "unknown"]);
export const staleReasons = new Set([
  "build_generation",
  "snapshot_generation",
  "fingerprints",
  "unknown",
]);
export const publications = new Set([
  "not_attempted",
  "published",
  "partial",
  "indeterminate",
  "refused",
  "unknown",
]);
export const notAttemptedReasons = new Set([
  "build_failed",
  "cancelled",
  "superseded",
  "stale",
  "no_candidates",
  "preparation_failed",
  "invalid_outcome",
  "unknown",
]);
export const refusalReasons = new Set([
  "stale_build_generation",
  "stale_snapshot_generation",
  "stale_fingerprints",
  "request_identity_mismatch",
  "unknown",
]);
export const recoveryReasons = new Set([
  "stage_cleanup_failed",
  "publication_indeterminate",
  "publication_uncommitted",
]);
export const recoveryIoKinds = new Set([
  "already_exists",
  "invalid_input",
  "not_found",
  "permission_denied",
  "other",
]);
export const failures = new Set([
  "check",
  "diagnostics",
  "engine",
  "duplicate_target",
  "preparation",
  "invalid_publication",
  "freshness",
  "unknown",
]);
export const checkReasons = new Set(["request_mismatch", "freshness_mismatch", "unknown"]);
export const engineReasons = new Set(["invalid_output", "host", "unknown"]);
export const publishFailureReasons = new Set(["rejected", "storage", "unknown"]);
export { errorCategories, errorCodes, operations } from "./error-vocabulary.generated.js";
