use super::schema_contract as contract;
use crate::DiagnosticPresentationContract;

contract!(JSON_PARSE, "RECITE_SCHEMA001", "diagnostic-schema-001-json-parse", ["detail" => String]);
contract!(TOML_PARSE, "RECITE_SCHEMA001", "diagnostic-schema-001-toml-parse", ["detail" => String]);
contract!(TOML_DECODE, "RECITE_SCHEMA001", "diagnostic-schema-001-toml-decode", ["detail" => String]);
contract!(
    NON_FINITE,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-source-non-finite",
    []
);
contract!(
    LEGACY_BINDING,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-source-legacy-binding",
    []
);
contract!(TAGGED_FIELD, "RECITE_SCHEMA001", "diagnostic-schema-001-source-tagged-field", ["name" => String]);
contract!(GENERATED_FIELD, "RECITE_SCHEMA001", "diagnostic-schema-001-source-generated-field", ["key" => String]);
contract!(
    PRODUCER_REQUIRED,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-source-producer-required",
    []
);
contract!(
    PRODUCER_ID_REQUIRED,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-source-producer-id-required",
    []
);
contract!(
    PRODUCER_ID_EMPTY,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-source-producer-id-empty",
    []
);
contract!(
    PRODUCER_KIND,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-source-producer-kind",
    []
);
contract!(READ, "RECITE_SCHEMA001", "diagnostic-schema-001-read", ["detail" => String]);

const CONTRACTS: &[&DiagnosticPresentationContract] = &[
    &JSON_PARSE,
    &TOML_PARSE,
    &TOML_DECODE,
    &NON_FINITE,
    &LEGACY_BINDING,
    &TAGGED_FIELD,
    &GENERATED_FIELD,
    &PRODUCER_REQUIRED,
    &PRODUCER_ID_REQUIRED,
    &PRODUCER_ID_EMPTY,
    &PRODUCER_KIND,
    &READ,
];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}
