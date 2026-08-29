use super::schema_contract as contract;
use crate::DiagnosticPresentationContract;

contract!(UNSUPPORTED_VERSION, "RECITE_SCHEMA002", "diagnostic-schema-002-unsupported-version", ["version" => String]);
contract!(
    SCHEMA_VERSION_TYPE,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-schema-version-type",
    []
);
contract!(
    FLOAT_NOT_REPRESENTABLE,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-float-not-representable",
    ["owner" => String]
);
contract!(
    PRODUCER_EXPORT_VERSION,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-producer-export-version",
    []
);
contract!(
    PRODUCER_FINGERPRINT_EMPTY_ALGORITHM,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-producer-content-fingerprint-empty-algorithm",
    []
);
contract!(
    PRODUCER_FINGERPRINT_BLAKE3_HEX_SHAPE,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-producer-content-fingerprint-blake3-hex-shape",
    []
);
contract!(
    PRODUCER_FINGERPRINT_BLAKE3_HEX_DATA,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-producer-content-fingerprint-blake3-hex-data",
    []
);
contract!(
    PRODUCER_FINGERPRINT_EMPTY_DIGEST,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-producer-content-fingerprint-empty-digest",
    []
);
contract!(PRODUCER_FINGERPRINT_BLAKE3_DIGEST_LENGTH, "RECITE_SCHEMA001", "diagnostic-schema-001-producer-content-fingerprint-blake3-digest-length", ["actual" => Integer]);
contract!(ORIGIN_EXTENSION, "RECITE_SCHEMA001", "diagnostic-schema-001-origin-extension", ["owner" => String, "key" => String]);
contract!(VALUE_ORIGINS, "RECITE_SCHEMA001", "diagnostic-schema-001-value-origins", ["owner" => String]);
contract!(PRODUCER_FINGERPRINT, "RECITE_SCHEMA003", "diagnostic-schema-003-producer-fingerprint", ["owner" => String, "kind" => String, "id" => String]);
contract!(PROVENANCE_UNKNOWN_VALUE, "RECITE_SCHEMA001", "diagnostic-schema-001-provenance-unknown-value", ["owner" => String, "key" => String]);
contract!(TYPE_KIND, "RECITE_SCHEMA001", "diagnostic-schema-001-type-kind", ["type" => String, "kind" => String]);
contract!(VALUE, "RECITE_SCHEMA003", "diagnostic-schema-003-value", ["owner" => String, "value" => String]);
contract!(METADATA_TARGET, "RECITE_SCHEMA001", "diagnostic-schema-001-metadata-target", ["metadata" => String, "target" => String]);
contract!(METADATA_TARGET_DUPLICATE, "RECITE_SCHEMA003", "diagnostic-schema-003-metadata-target", ["metadata" => String, "target" => String]);
contract!(METADATA_ARRAY_TYPE, "RECITE_SCHEMA001", "diagnostic-schema-001-metadata-array-type", ["metadata" => String, "type_ref" => String]);
contract!(METADATA_DOMAIN_TYPE, "RECITE_SCHEMA001", "diagnostic-schema-001-metadata-domain-type", ["metadata" => String, "type_ref" => String]);
contract!(CONDITION_RETURN, "RECITE_SCHEMA004", "diagnostic-schema-004-invalid-condition-return", ["condition" => String, "return_type" => String]);
contract!(EFFECT_MODE, "RECITE_SCHEMA001", "diagnostic-schema-001-effect-mode", ["effect" => String, "mode" => String]);
contract!(EFFECT_MODE_DUPLICATE, "RECITE_SCHEMA003", "diagnostic-schema-003-effect-mode", ["effect" => String, "mode" => String]);
contract!(PARAMETER_DUPLICATE, "RECITE_SCHEMA003", "diagnostic-schema-003-parameter", ["owner" => String, "parameter" => String]);
contract!(PARAMETER_SPECIAL_TYPE, "RECITE_SCHEMA004", "diagnostic-schema-004-parameter-special-type", ["owner" => String, "parameter" => String, "type_ref" => String]);
contract!(INVALID_METADATA_TYPE, "RECITE_SCHEMA004", "diagnostic-schema-004-invalid-metadata-type", ["metadata" => String, "type_ref" => String]);
contract!(INVALID_PARAMETER_TYPE, "RECITE_SCHEMA004", "diagnostic-schema-004-invalid-parameter-type", ["parameter" => String, "type_ref" => String]);
contract!(INVALID_PROJECTION_INPUT_TYPE, "RECITE_SCHEMA004", "diagnostic-schema-004-invalid-projection-input-type", ["projector" => String, "input" => String, "type_ref" => String]);
contract!(INVALID_PROJECTION_OUTPUT_TYPE, "RECITE_SCHEMA004", "diagnostic-schema-004-invalid-projection-output-type", ["projector" => String, "output" => String, "binding" => String, "type_ref" => String]);
contract!(INVALID_QUERY_RETURN_TYPE, "RECITE_SCHEMA004", "diagnostic-schema-004-invalid-query-return-type", ["function" => String, "type_ref" => String]);
contract!(CONTEXTUAL_DOMAIN_FOR_FLAT, "RECITE_SCHEMA004", "diagnostic-schema-004-contextual-domain-for-flat", ["owner" => String, "domain" => String]);
contract!(UNKNOWN_METADATA_DOMAIN, "RECITE_SCHEMA004", "diagnostic-schema-004-unknown-metadata-domain", ["owner" => String, "domain" => String]);
contract!(UNKNOWN_ENUM, "RECITE_SCHEMA004", "diagnostic-schema-004-unknown-enum", ["owner" => String, "name" => String]);
contract!(UNKNOWN_REGISTRY, "RECITE_SCHEMA004", "diagnostic-schema-004-unknown-registry", ["owner" => String, "name" => String]);
contract!(DUPLICATE_DEFINITION, "RECITE_SCHEMA003", "diagnostic-schema-003-duplicate-definition", ["kind" => String, "name" => String]);
contract!(EMPTY_VALUE, "RECITE_SCHEMA001", "diagnostic-schema-001-empty-value", ["field" => String]);
contract!(INVALID_NAME, "RECITE_SCHEMA001", "diagnostic-schema-001-invalid-name", ["field" => String]);

const CONTRACTS: &[&DiagnosticPresentationContract] = &[
    &UNSUPPORTED_VERSION,
    &SCHEMA_VERSION_TYPE,
    &FLOAT_NOT_REPRESENTABLE,
    &PRODUCER_EXPORT_VERSION,
    &PRODUCER_FINGERPRINT_EMPTY_ALGORITHM,
    &PRODUCER_FINGERPRINT_BLAKE3_HEX_SHAPE,
    &PRODUCER_FINGERPRINT_BLAKE3_HEX_DATA,
    &PRODUCER_FINGERPRINT_EMPTY_DIGEST,
    &PRODUCER_FINGERPRINT_BLAKE3_DIGEST_LENGTH,
    &ORIGIN_EXTENSION,
    &VALUE_ORIGINS,
    &PRODUCER_FINGERPRINT,
    &PROVENANCE_UNKNOWN_VALUE,
    &TYPE_KIND,
    &VALUE,
    &METADATA_TARGET,
    &METADATA_TARGET_DUPLICATE,
    &METADATA_ARRAY_TYPE,
    &METADATA_DOMAIN_TYPE,
    &CONDITION_RETURN,
    &EFFECT_MODE,
    &EFFECT_MODE_DUPLICATE,
    &PARAMETER_DUPLICATE,
    &PARAMETER_SPECIAL_TYPE,
    &INVALID_METADATA_TYPE,
    &INVALID_PARAMETER_TYPE,
    &INVALID_PROJECTION_INPUT_TYPE,
    &INVALID_PROJECTION_OUTPUT_TYPE,
    &INVALID_QUERY_RETURN_TYPE,
    &CONTEXTUAL_DOMAIN_FOR_FLAT,
    &UNKNOWN_METADATA_DOMAIN,
    &UNKNOWN_ENUM,
    &UNKNOWN_REGISTRY,
    &DUPLICATE_DEFINITION,
    &EMPTY_VALUE,
    &INVALID_NAME,
];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}
