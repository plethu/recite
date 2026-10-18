import * as v from "valibot";
import {
  nonemptyString,
  positiveInteger,
  unsignedInteger,
  signedInteger,
  wideUnsignedInteger,
} from "./common.ts";

const scalar = v.variant("type", [
  v.strictObject({ type: v.picklist(["identifier", "string"]), value: v.string() }),
  v.strictObject({ type: v.literal("integer"), value: signedInteger }),
  v.strictObject({ type: v.literal("float"), value: v.pipe(v.number(), v.finite()) }),
  v.strictObject({ type: v.literal("boolean"), value: v.boolean() }),
]);
const metadata = v.strictObject({
  key: nonemptyString,
  value: v.variant("type", [
    v.strictObject({ type: v.literal("scalar"), value: scalar }),
    v.strictObject({ type: v.literal("array"), value: v.array(scalar) }),
  ]),
});
const origin = v.variant("type", [
  v.strictObject({
    type: v.literal("condition_call"),
    function: nonemptyString,
    args: v.array(scalar),
  }),
  v.strictObject({ type: v.literal("requirement_expression"), source_text: v.string() }),
]);
const availabilityReason = v.strictObject({
  id: nonemptyString,
  source_text: v.string(),
  localized_template: v.string(),
  text: v.string(),
  origin: v.nullable(origin),
  args: v.array(v.strictObject({ name: nonemptyString, value: scalar })),
});
type ReasonTree =
  | { type: "all" | "any"; value: ReasonTree[] }
  | { type: "reason"; value: v.InferInput<typeof availabilityReason> }
  | { type: "requirement_source_text"; value: string };
const reasonTree: v.GenericSchema<ReasonTree> = v.lazy(() => reasonTreeVariants);
const reasonTreeVariants = v.variant("type", [
  v.strictObject({ type: v.picklist(["all", "any"]), value: v.array(reasonTree) }),
  v.strictObject({ type: v.literal("reason"), value: availabilityReason }),
  v.strictObject({ type: v.literal("requirement_source_text"), value: v.string() }),
]);
const pluralAttempt = v.strictObject({
  locale: nonemptyString,
  context: v.string(),
  key: v.string(),
  selected_arm: v.nullable(unsignedInteger),
  outcome: v.picklist(["missing_plural_forms", "missing_entry", "missing_translation", "matched"]),
});
const plural = v.strictObject({
  singular_source_text: v.string(),
  plural_source_text: v.string(),
  count: signedInteger,
  selected_arm: unsignedInteger,
  attempts: v.array(pluralAttempt),
  matched_locale: v.nullable(nonemptyString),
  matched_context: v.nullable(nonemptyString),
  matched_key: v.nullable(nonemptyString),
  matched_arm: v.nullable(unsignedInteger),
  source_fallback_arm: v.nullable(unsignedInteger),
  outcome: v.picklist(["translated", "english_source_fallback"]),
});
const line = v.strictObject({
  id: nonemptyString,
  source_text: v.string(),
  text: v.string(),
  speaker: v.nullable(nonemptyString),
  metadata: v.array(metadata),
  plural: v.optional(plural),
});
const promptIdentity = v.strictObject({
  block: nonemptyString,
  line: v.nullable(nonemptyString),
  fixture_keys: v.array(nonemptyString),
});
const choice = v.strictObject({
  id: nonemptyString,
  source_text: v.string(),
  text: v.string(),
  metadata: v.array(metadata),
  is_available: v.boolean(),
  availability: v.strictObject({
    is_available: v.boolean(),
    primary_reason: v.nullable(availabilityReason),
    reason_tree: v.nullable(reasonTree),
  }),
  unavailable_reason: v.nullable(nonemptyString),
});
const prompt = v.strictObject({
  identity: promptIdentity,
  line: v.nullable(line),
  choices: v.array(choice),
});
const sourceSpan = v.pipe(
  v.strictObject({
    file: v.string(),
    start_line: positiveInteger,
    start_column: positiveInteger,
    end_line: v.nullable(positiveInteger),
    end_column: v.nullable(positiveInteger),
  }),
  v.check((span) => (span.end_line === null) === (span.end_column === null)),
);
const effect = v.strictObject({
  id: nonemptyString,
  mode: v.picklist(["deferred", "immediate", "blocking"]),
  function: nonemptyString,
  args: v.array(scalar),
  source_span: sourceSpan,
});
const condition = v.strictObject({
  query: nonemptyString,
  function: nonemptyString,
  arguments: v.array(scalar),
  result: v.union([v.boolean(), v.strictObject({ enum: nonemptyString })]),
});
const event = v.variant("type", [
  v.strictObject({ type: v.literal("condition"), condition }),
  v.strictObject({ type: v.literal("line"), line }),
  v.strictObject({ type: v.literal("prompt"), prompt }),
  v.strictObject({
    type: v.literal("choice_selected"),
    prompt: promptIdentity,
    choice: nonemptyString,
  }),
  v.strictObject({ type: v.literal("effect"), effect }),
  v.strictObject({
    type: v.literal("acknowledgement"),
    effect_id: nonemptyString,
    result: v.literal("completed"),
  }),
  v.strictObject({ type: v.literal("end"), deferred_effects: v.array(effect) }),
]);
const metrics = v.strictObject({
  event_count: unsignedInteger,
  line_count: unsignedInteger,
  prompt_count: unsignedInteger,
  choice_count: unsignedInteger,
  condition_evaluation_count: unsignedInteger,
  effect_count: v.strictObject({
    deferred: unsignedInteger,
    immediate: unsignedInteger,
    blocking: unsignedInteger,
  }),
  localization_lookup_count: unsignedInteger,
  elapsed_traversal_time_ns: wideUnsignedInteger,
  max_serialized_session_size_bytes: unsignedInteger,
});
export const traceSchema = v.strictObject({
  asset_id: nonemptyString,
  block: nonemptyString,
  events: v.array(event),
  final_deferred_effects: v.array(effect),
  dialogue_locale: v.optional(nonemptyString),
  dialogue_locale_fallbacks: v.optional(v.array(nonemptyString)),
  metrics: v.optional(metrics),
});
export type Trace = v.InferInput<typeof traceSchema>;
