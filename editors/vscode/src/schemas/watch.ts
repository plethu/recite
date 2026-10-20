import * as v from "valibot";
import { errorCategories, errorCodes, operations } from "../error-vocabulary.generated.js";
import {
  artifactSchema,
  machinePathSchema,
  nonemptyString,
  sortedStrings,
  uniqueStrings,
  unsignedInteger,
  compareUtf8,
} from "./common.ts";
import { diagnosticRecordSchema } from "./diagnostic.ts";

const cancellationSchema = v.variant("type", [
  v.strictObject({ type: v.picklist(["user", "unknown"]) }),
  v.strictObject({ type: v.literal("superseded"), by_generation: unsignedInteger }),
]);
const freshnessSchema = v.variant("type", [
  v.strictObject({ type: v.picklist(["fresh", "unknown"]) }),
  v.strictObject({
    type: v.literal("stale"),
    reasons: v.pipe(
      v.array(
        v.strictObject({
          type: v.picklist(["build_generation", "snapshot_generation", "fingerprints", "unknown"]),
        }),
      ),
      v.check((values) =>
        values.every(
          (value, index) => index === 0 || compareUtf8(values[index - 1]!.type, value.type) < 0,
        ),
      ),
    ),
  }),
]);
const outcomeSchema = v.strictObject({
  type: v.picklist([
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
  ]),
});
const publicationSchema = v.variant("type", [
  v.strictObject({
    type: v.literal("not_attempted"),
    reason: v.picklist([
      "build_failed",
      "cancelled",
      "superseded",
      "stale",
      "no_candidates",
      "preparation_failed",
      "invalid_outcome",
      "unknown",
    ]),
  }),
  v.strictObject({ type: v.literal("published"), targets: sortedStrings }),
  v.strictObject({
    type: v.literal("partial"),
    committed: sortedStrings,
    failed: nonemptyString,
    remaining: sortedStrings,
    recovery: sortedStrings,
  }),
  v.strictObject({
    type: v.literal("indeterminate"),
    attempted: sortedStrings,
    recovery: sortedStrings,
  }),
  v.strictObject({
    type: v.literal("refused"),
    reason: v.picklist([
      "stale_build_generation",
      "stale_snapshot_generation",
      "stale_fingerprints",
      "request_identity_mismatch",
      "unknown",
    ]),
  }),
  v.strictObject({ type: v.literal("unknown") }),
]);
export const recoverySchema = v.strictObject({
  marker: machinePathSchema,
  reason: v.picklist([
    "stage_cleanup_failed",
    "publication_indeterminate",
    "publication_uncommitted",
  ]),
  detail: v.optional(
    v.strictObject({
      type: v.literal("io"),
      kind: v.picklist([
        "already_exists",
        "invalid_input",
        "not_found",
        "permission_denied",
        "other",
      ]),
      raw_os_error: v.nullable(v.pipe(v.number(), v.safeInteger())),
    }),
  ),
});
const restartGuidanceSchema = v.strictObject({
  type: v.literal("host_policy_required"),
  decision: v.literal("unspecified"),
});
const errorDetailsSchema = v.variant("type", [
  v.strictObject({
    type: v.literal("fixture_choice"),
    choice: nonemptyString,
    prompt_keys: uniqueStrings,
  }),
  v.strictObject({
    type: v.literal("fixture_choice_index"),
    index: unsignedInteger,
    choice_count: unsignedInteger,
    prompt_keys: uniqueStrings,
  }),
  v.strictObject({
    type: v.literal("ambiguous_fixture"),
    block: nonemptyString,
    prompt_count: unsignedInteger,
  }),
  v.strictObject({ type: v.literal("missing_fixture_choice"), prompt_keys: uniqueStrings }),
  v.strictObject({ type: v.literal("blocking_effect"), effect: nonemptyString }),
  v.strictObject({ type: v.literal("locale"), field: nonemptyString, locale: nonemptyString }),
  v.strictObject({ type: v.literal("catalog_spec"), spec: nonemptyString }),
  v.strictObject({ type: v.literal("watch"), kind: nonemptyString }),
  v.strictObject({ type: v.literal("watch_target"), kind: nonemptyString, target: nonemptyString }),
]);
export const structuredErrorSchema = v.strictObject({
  category: v.pipe(
    v.string(),
    v.check((value) => errorCategories.has(value)),
  ),
  code: v.pipe(
    v.string(),
    v.check((value) => errorCodes.has(value)),
  ),
  operation: v.pipe(
    v.string(),
    v.check((value) => operations.has(value)),
  ),
  path: v.optional(machinePathSchema),
  related_path: v.optional(machinePathSchema),
  details: v.optional(errorDetailsSchema),
});
const failureSchema = v.variant("type", [
  v.strictObject({
    type: v.literal("check"),
    reason: v.picklist(["request_mismatch", "freshness_mismatch", "unknown"]),
  }),
  v.strictObject({
    type: v.literal("engine"),
    reason: v.picklist(["invalid_output", "host", "unknown"]),
  }),
  v.strictObject({ type: v.literal("duplicate_target"), target: nonemptyString }),
  v.strictObject({
    type: v.literal("preparation"),
    target: nonemptyString,
    reason: v.picklist(["rejected", "storage", "unknown"]),
  }),
  v.strictObject({
    type: v.picklist(["diagnostics", "invalid_publication", "freshness", "unknown"]),
  }),
]);
const buildStartSchema = v.strictObject({
  generation: unsignedInteger,
  trigger: v.picklist(["initial", "input_changed"]),
});
const outcomesByStatus = {
  succeeded: new Set<string>(["fresh", "recovery_required"]),
  failed: new Set<string>([
    "diagnostics",
    "recovery_required",
    "freshness_failure",
    "operational_failure",
    "publication_failure",
  ]),
  stale: new Set<string>(["stale"]),
  cancelled: new Set<string>(["cancelled"]),
  superseded: new Set<string>(["superseded"]),
  unknown: new Set<string>(["unknown"]),
};
const buildCompletedSchema = v.pipe(
  v.strictObject({
    generation: unsignedInteger,
    snapshot_generation: v.nullable(unsignedInteger),
    status: v.picklist(["succeeded", "failed", "stale", "cancelled", "superseded", "unknown"]),
    outcome: outcomeSchema,
    inputs: sortedStrings,
    diagnostics: v.array(diagnosticRecordSchema),
    artifacts: v.array(artifactSchema),
    freshness: freshnessSchema,
    publication: publicationSchema,
    recovery: v.array(recoverySchema),
    restart_guidance: restartGuidanceSchema,
    cancellation: v.optional(cancellationSchema),
    failure: v.optional(failureSchema),
    error: v.optional(structuredErrorSchema),
  }),
  v.check(
    (value) =>
      outcomesByStatus[value.status].has(value.outcome.type) &&
      (value.status !== "cancelled" || value.cancellation?.type === "user") &&
      (value.status !== "superseded" || value.cancellation?.type === "superseded") &&
      (value.outcome.type !== "diagnostics" || value.failure?.type === "diagnostics"),
  ),
);
const watchStopSchema = v.union([
  v.strictObject({
    reason: v.strictObject({ type: v.literal("cancelled") }),
    error: v.optional(v.undefined()),
  }),
  v.strictObject({
    reason: v.strictObject({ type: v.literal("fatal") }),
    error: structuredErrorSchema,
  }),
]);
const envelope = {
  version: v.literal(1),
  sequence: unsignedInteger,
  command: v.literal("watch"),
  invocation_id: v.optional(v.string()),
};
export const watchRecordSchema = v.variant("event", [
  v.strictObject({
    ...envelope,
    event: v.literal("watch.started"),
    data: v.strictObject({ project_root: machinePathSchema }),
  }),
  v.strictObject({ ...envelope, event: v.literal("watch.build.started"), data: buildStartSchema }),
  v.strictObject({
    ...envelope,
    event: v.literal("watch.build.completed"),
    data: buildCompletedSchema,
  }),
  v.strictObject({ ...envelope, event: v.literal("watch.waiting"), data: v.strictObject({}) }),
  v.strictObject({
    ...envelope,
    event: v.literal("watch.cancel.requested"),
    data: v.strictObject({ cancellation: v.strictObject({ type: v.literal("user") }) }),
  }),
  v.strictObject({
    ...envelope,
    event: v.literal("watch.control.error"),
    data: v.strictObject({
      error: v.strictObject({
        type: v.picklist([
          "malformed",
          "unsupported_version",
          "unsupported_command",
          "unsupported_action",
          "invocation_mismatch",
        ]),
      }),
    }),
  }),
  v.strictObject({
    ...envelope,
    event: v.literal("watch.notify.error"),
    data: v.strictObject({ error: v.strictObject({ type: v.literal("watcher") }) }),
  }),
  v.strictObject({ ...envelope, event: v.literal("watch.stopped"), data: watchStopSchema }),
]);
export type WatchRecord = v.InferInput<typeof watchRecordSchema>;
export type MachinePath = v.InferInput<typeof machinePathSchema>;
export type StructuredError = v.InferInput<typeof structuredErrorSchema>;
