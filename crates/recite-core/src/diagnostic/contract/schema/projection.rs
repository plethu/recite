use super::schema_contract as contract;
use crate::DiagnosticPresentationContract;

contract!(CANDIDATE_SOURCE, "RECITE_SCHEMA001", "diagnostic-schema-001-projection-candidate-source", ["projector" => String, "input" => String]);
contract!(CANDIDATE_NO_TARGET, "RECITE_SCHEMA001", "diagnostic-schema-001-projection-candidate-no-target", ["projector" => String, "input" => String]);
contract!(OCCURRENCE_REPEAT, "RECITE_SCHEMA001", "diagnostic-schema-001-projection-occurrence-repeat", ["projector" => String, "input" => String, "occurrence" => String, "key" => String]);
contract!(OCCURRENCE_ALL_TYPE, "RECITE_SCHEMA001", "diagnostic-schema-001-projection-occurrence-all-type", ["projector" => String, "input" => String, "type_ref" => String]);
contract!(CANDIDATE_TYPE_MISMATCH, "RECITE_SCHEMA001", "diagnostic-schema-001-projection-candidate-type-mismatch", ["projector" => String, "input" => String, "expected" => String, "key" => String, "actual" => String]);
contract!(OCCURRENCE_ARRAY, "RECITE_SCHEMA001", "diagnostic-schema-001-projection-occurrence-array", ["projector" => String, "input" => String, "type_ref" => String]);
contract!(REASON_NO_SELECTOR, "RECITE_SCHEMA001", "diagnostic-schema-001-projection-reason-no-selector", ["projector" => String, "input" => String]);
contract!(REASON_ARG, "RECITE_SCHEMA004", "diagnostic-schema-004-projection-reason-arg", ["projector" => String, "input" => String, "name" => String]);
contract!(REASON_TYPE, "RECITE_SCHEMA001", "diagnostic-schema-001-projection-reason-type", ["projector" => String, "input" => String, "name" => String, "expected" => String, "actual" => String]);
contract!(OCCURRENCE, "RECITE_SCHEMA001", "diagnostic-schema-001-projection-occurrence", ["projector" => String, "input" => String, "name" => String]);
contract!(FIELD_DUPLICATE, "RECITE_SCHEMA003", "diagnostic-schema-003-projection-field", ["projector" => String, "output" => String, "field" => String]);
contract!(INPUT_DUPLICATE, "RECITE_SCHEMA003", "diagnostic-schema-003-projection-input", ["projector" => String, "input" => String]);
contract!(LABEL_TEMPLATE_DUPLICATE, "RECITE_SCHEMA003", "diagnostic-schema-003-label-template", ["template_id" => String]);
contract!(LABEL_ARGUMENT_DUPLICATE, "RECITE_SCHEMA003", "diagnostic-schema-003-label-argument", ["projector" => String, "output" => String, "argument" => String]);
contract!(LABEL_UNTERMINATED, "RECITE_SCHEMA001", "diagnostic-schema-001-label-placeholder-unterminated", ["projector" => String, "output" => String, "template_id" => String]);
contract!(LABEL_INVALID_NAME, "RECITE_SCHEMA001", "diagnostic-schema-001-label-placeholder-invalid-name", ["projector" => String, "output" => String, "template_id" => String, "name" => String]);
contract!(LABEL_UNESCAPED_CLOSING_BRACE, "RECITE_SCHEMA001", "diagnostic-schema-001-label-placeholder-unescaped-closing-brace", ["projector" => String, "output" => String, "template_id" => String]);
contract!(LABEL_UNKNOWN_ARG, "RECITE_SCHEMA001", "diagnostic-schema-001-label-unknown-arg", ["projector" => String, "output" => String, "template_id" => String, "placeholder" => String]);
contract!(LABEL_UNUSED_ARG, "RECITE_SCHEMA001", "diagnostic-schema-001-label-unused-arg", ["projector" => String, "output" => String, "template_id" => String, "arg" => String]);
contract!(LITERAL_INT, "RECITE_SCHEMA001", "diagnostic-schema-001-projection-literal-int", ["owner" => String]);
contract!(LITERAL_TYPE, "RECITE_SCHEMA001", "diagnostic-schema-001-projection-literal-type", ["owner" => String, "expected" => String, "actual" => String]);
contract!(LITERAL_UNKNOWN, "RECITE_SCHEMA001", "diagnostic-schema-001-projection-literal-unknown", ["owner" => String, "expected" => String, "value" => String]);
contract!(OUTPUT_DUPLICATE, "RECITE_SCHEMA003", "diagnostic-schema-003-projection-output", ["projector" => String, "output" => String]);
contract!(OUTPUT_TARGET, "RECITE_SCHEMA001", "diagnostic-schema-001-projection-output-target", ["projector" => String, "output" => String, "target" => String]);
contract!(QUERY_MAX_CALLS, "RECITE_SCHEMA001", "diagnostic-schema-001-query-max-calls", ["function" => String]);
contract!(QUERY_DUPLICATE, "RECITE_SCHEMA003", "diagnostic-schema-003-projection-query", ["projector" => String, "query" => String]);
contract!(UNKNOWN_QUERY_FUNCTION, "RECITE_SCHEMA004", "diagnostic-schema-004-unknown-query-function", ["projector" => String, "query" => String, "function" => String]);
contract!(QUERY_ARG_COUNT, "RECITE_SCHEMA001", "diagnostic-schema-001-query-arg-count", ["projector" => String, "query" => String, "actual" => Integer, "function" => String, "expected" => Integer]);
contract!(UNKNOWN_REF, "RECITE_SCHEMA004", "diagnostic-schema-004-unknown-projection-ref", ["projector" => String, "owner" => String, "ref" => String]);
contract!(REF_TYPE, "RECITE_SCHEMA001", "diagnostic-schema-001-projection-ref-type", ["projector" => String, "owner" => String, "expected" => String, "ref" => String, "actual" => String]);
contract!(REQUIRED_METADATA_DUPLICATE, "RECITE_SCHEMA003", "diagnostic-schema-003-required-metadata", ["projector" => String, "key" => String]);
contract!(UNKNOWN_REASON_SELECTOR, "RECITE_SCHEMA004", "diagnostic-schema-004-unknown-projection-reason", ["projector" => String, "reason" => String]);
contract!(SELECTOR_TARGET, "RECITE_SCHEMA001", "diagnostic-schema-001-projection-selector-target", ["target" => String]);
contract!(UNKNOWN_METADATA_KEY, "RECITE_SCHEMA004", "diagnostic-schema-004-unknown-metadata-key", ["projector" => String, "key" => String]);
contract!(METADATA_TARGET, "RECITE_SCHEMA001", "diagnostic-schema-001-projection-metadata-target", ["projector" => String, "key" => String, "target" => String]);

const CONTRACTS: &[&DiagnosticPresentationContract] = &[
    &CANDIDATE_SOURCE,
    &CANDIDATE_NO_TARGET,
    &OCCURRENCE_REPEAT,
    &OCCURRENCE_ALL_TYPE,
    &CANDIDATE_TYPE_MISMATCH,
    &OCCURRENCE_ARRAY,
    &REASON_NO_SELECTOR,
    &REASON_ARG,
    &REASON_TYPE,
    &OCCURRENCE,
    &FIELD_DUPLICATE,
    &INPUT_DUPLICATE,
    &LABEL_TEMPLATE_DUPLICATE,
    &LABEL_ARGUMENT_DUPLICATE,
    &LABEL_UNTERMINATED,
    &LABEL_INVALID_NAME,
    &LABEL_UNESCAPED_CLOSING_BRACE,
    &LABEL_UNKNOWN_ARG,
    &LABEL_UNUSED_ARG,
    &LITERAL_INT,
    &LITERAL_TYPE,
    &LITERAL_UNKNOWN,
    &OUTPUT_DUPLICATE,
    &OUTPUT_TARGET,
    &QUERY_MAX_CALLS,
    &QUERY_DUPLICATE,
    &UNKNOWN_QUERY_FUNCTION,
    &QUERY_ARG_COUNT,
    &UNKNOWN_REF,
    &REF_TYPE,
    &REQUIRED_METADATA_DUPLICATE,
    &UNKNOWN_REASON_SELECTOR,
    &SELECTOR_TARGET,
    &UNKNOWN_METADATA_KEY,
    &METADATA_TARGET,
];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}
