import { integerInRange } from "./command-protocol.js";

const I64_MIN = "-9223372036854775808";
const I64_MAX = "9223372036854775807";
const U64_MAX = "18446744073709551615";
const U128_MAX = "340282366920938463463374607431768211455";

/** Validate the closed v1 runtime trace model before semantic UI projection. */
export function validTrace(value) {
  return object(value) && keys(value, ["asset_id", "block", "events", "final_deferred_effects"],
    ["dialogue_locale", "dialogue_locale_fallbacks", "metrics"]) &&
    string(value.asset_id) && string(value.block) &&
    optional(value.dialogue_locale, string) && optional(value.dialogue_locale_fallbacks,
      (items) => array(items, string)) && array(value.events, validEvent) &&
    array(value.final_deferred_effects, validEffect) && optional(value.metrics, validMetrics);
}

function validEvent(value) {
  if (!object(value) || typeof value.type !== "string") return false;
  switch (value.type) {
    case "condition": return keys(value, ["type", "condition"]) && validCondition(value.condition);
    case "line": return keys(value, ["type", "line"]) && validLine(value.line);
    case "prompt": return keys(value, ["type", "prompt"]) && validPrompt(value.prompt);
    case "choice_selected": return keys(value, ["type", "prompt", "choice"]) &&
      validPromptIdentity(value.prompt) && string(value.choice);
    case "effect": return keys(value, ["type", "effect"]) && validEffect(value.effect);
    case "acknowledgement": return keys(value, ["type", "effect_id", "result"]) &&
      string(value.effect_id) && value.result === "completed";
    case "end": return keys(value, ["type", "deferred_effects"]) &&
      array(value.deferred_effects, validEffect);
    default: return false;
  }
}

function validCondition(value) {
  return object(value) && keys(value, ["query", "function", "arguments", "result"]) &&
    string(value.query) && string(value.function) && array(value.arguments, validScalar) &&
    (typeof value.result === "boolean" || object(value.result) && keys(value.result, ["enum"]) &&
      string(value.result.enum));
}

function validPrompt(value) {
  return object(value) && keys(value, ["identity", "line", "choices"]) &&
    validPromptIdentity(value.identity) && nullable(value.line, validLine) &&
    array(value.choices, validChoice);
}

function validPromptIdentity(value) {
  return object(value) && keys(value, ["block", "line", "fixture_keys"]) && string(value.block) &&
    nullable(value.line, string) && array(value.fixture_keys, string);
}

function validLine(value) {
  return object(value) && keys(value, ["id", "source_text", "text", "speaker", "metadata"],
    ["plural"]) && string(value.id) && stringValue(value.source_text) &&
    stringValue(value.text) && nullable(value.speaker, string) && array(value.metadata, validMetadata) &&
    optional(value.plural, validPlural);
}

function validPlural(value) {
  return object(value) && keys(value, ["singular_source_text", "plural_source_text", "count", "selected_arm",
    "attempts", "matched_locale", "matched_context", "matched_key", "matched_arm", "source_fallback_arm", "outcome"],
    ["matched_locale", "matched_context", "matched_key", "matched_arm", "source_fallback_arm"]) &&
    stringValue(value.singular_source_text) && stringValue(value.plural_source_text) &&
    integerInRange(value.count, I64_MIN, I64_MAX) && integerInRange(value.selected_arm, "0", U64_MAX) &&
    array(value.attempts, validPluralAttempt) && nullable(value.matched_locale, string) &&
    nullable(value.matched_context, string) && nullable(value.matched_key, string) &&
    nullable(value.matched_arm, (arm) => integerInRange(arm, "0", U64_MAX)) &&
    nullable(value.source_fallback_arm, (arm) => integerInRange(arm, "0", U64_MAX)) &&
    ["translated", "english_source_fallback"].includes(value.outcome);
}

function validPluralAttempt(value) {
  return object(value) && keys(value, ["locale", "context", "key", "selected_arm", "outcome"], ["selected_arm"]) &&
    string(value.locale) && stringValue(value.context) && stringValue(value.key) &&
    nullable(value.selected_arm, (arm) => integerInRange(arm, "0", U64_MAX)) &&
    ["missing_plural_forms", "missing_entry", "missing_translation", "matched"].includes(value.outcome);
}

function validChoice(value) {
  return object(value) && keys(value, ["id", "source_text", "text", "metadata", "is_available", "availability",
    "unavailable_reason"]) && string(value.id) && stringValue(value.source_text) && stringValue(value.text) &&
    array(value.metadata, validMetadata) && typeof value.is_available === "boolean" &&
    validAvailability(value.availability) && nullable(value.unavailable_reason, string);
}

function validAvailability(value) {
  return object(value) && keys(value, ["is_available", "primary_reason", "reason_tree"],
    ["primary_reason", "reason_tree"]) && typeof value.is_available === "boolean" &&
    nullable(value.primary_reason, validAvailabilityReason) &&
    nullable(value.reason_tree, validReasonTree);
}

function validAvailabilityReason(value) {
  return object(value) && keys(value, ["id", "source_text", "localized_template", "text", "origin", "args"],
    ["origin"]) && string(value.id) && stringValue(value.source_text) &&
    stringValue(value.localized_template) && stringValue(value.text) && nullable(value.origin, validOrigin) &&
    array(value.args, validReasonArg);
}

function validOrigin(value) {
  if (!object(value) || typeof value.type !== "string") return false;
  if (value.type === "condition_call") return keys(value, ["type", "function", "args"]) &&
    string(value.function) && array(value.args, validReasonValue);
  return value.type === "requirement_expression" && keys(value, ["type", "source_text"]) && stringValue(value.source_text);
}

function validReasonArg(value) {
  return object(value) && keys(value, ["name", "value"]) && string(value.name) && validReasonValue(value.value);
}

function validReasonValue(value) {
  return taggedValue(value, ["identifier", "string", "integer", "float", "boolean"], validTaggedScalar);
}

function validReasonTree(value) {
  if (!object(value) || typeof value.type !== "string") return false;
  if (["all", "any"].includes(value.type)) return keys(value, ["type", "value"]) && array(value.value, validReasonTree);
  if (value.type === "reason") return keys(value, ["type", "value"]) && validAvailabilityReason(value.value);
  return value.type === "requirement_source_text" && keys(value, ["type", "value"]) && stringValue(value.value);
}

function validMetadata(value) {
  return object(value) && keys(value, ["key", "value"]) && string(value.key) && validTraceValue(value.value);
}

function validTraceValue(value) {
  return taggedValue(value, ["scalar", "array"], (inner, type) =>
    type === "scalar" ? validScalar(inner) : array(inner, validScalar));
}

function validScalar(value) {
  return taggedValue(value, ["identifier", "string", "integer", "float", "boolean"], validTaggedScalar);
}

function validEffect(value) {
  return object(value) && keys(value, ["id", "mode", "function", "args", "source_span"]) && string(value.id) &&
    ["deferred", "immediate", "blocking"].includes(value.mode) && string(value.function) &&
    array(value.args, validScalar) && validSourceSpan(value.source_span);
}

function validSourceSpan(value) {
  return object(value) && keys(value, ["file", "start_line", "start_column", "end_line", "end_column"]) &&
    stringValue(value.file) && positive(value.start_line) && positive(value.start_column) &&
    nullable(value.end_line, positive) && nullable(value.end_column, positive) &&
    ((value.end_line === null) === (value.end_column === null));
}

function validMetrics(value) {
  return object(value) && keys(value, ["event_count", "line_count", "prompt_count", "choice_count",
    "condition_evaluation_count", "effect_count", "localization_lookup_count", "elapsed_traversal_time_ns",
    "max_serialized_session_size_bytes"]) && ["event_count", "line_count", "prompt_count", "choice_count",
    "condition_evaluation_count", "localization_lookup_count",
    "max_serialized_session_size_bytes"].every((key) => positiveOrZero(value[key])) &&
    integerInRange(value.elapsed_traversal_time_ns, "0", U128_MAX) &&
    object(value.effect_count) && keys(value.effect_count, ["deferred", "immediate", "blocking"]) &&
    ["deferred", "immediate", "blocking"].every((key) => positiveOrZero(value.effect_count[key]));
}

function taggedValue(value, types, check) {
  return object(value) && types.includes(value.type) && keys(value, ["type", "value"]) && check(value.value, value.type);
}

function object(value) { return Boolean(value && typeof value === "object" && !Array.isArray(value)); }
function string(value) { return typeof value === "string" && value.length > 0; }
function stringValue(value) { return typeof value === "string"; }
function array(value, predicate) { return Array.isArray(value) && value.every(predicate); }
function optional(value, predicate) { return value === undefined || predicate(value); }
function nullable(value, predicate) { return value === null || predicate(value); }
function positive(value) { return Number.isSafeInteger(value) && value > 0; }
function positiveOrZero(value) { return integerInRange(value, "0", U64_MAX); }
function validTaggedScalar(value, type) {
  if (["identifier", "string"].includes(type)) return typeof value === "string";
  if (type === "integer") return integerInRange(value, I64_MIN, I64_MAX);
  if (type === "float") return typeof value === "number" && Number.isFinite(value);
  return type === "boolean" && typeof value === "boolean";
}
function keys(value, required, optionalKeys = []) {
  const allowed = new Set([...required, ...optionalKeys]);
  return required.every((key) => Object.hasOwn(value, key)) && Object.keys(value).every((key) => allowed.has(key));
}
