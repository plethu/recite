use super::schema_contract as contract;
use crate::DiagnosticPresentationContract;

contract!(NON_BOOL_MAPPING, "RECITE_SCHEMA001", "diagnostic-schema-001-availability-non-bool-mapping", ["condition" => String]);
contract!(UNKNOWN_REASON, "RECITE_SCHEMA004", "diagnostic-schema-004-unknown-availability-reason", ["condition" => String, "reason" => String]);
contract!(TEMPLATE_UNTERMINATED, "RECITE_SCHEMA001", "diagnostic-schema-001-availability-template-unterminated", ["reason" => String]);
contract!(TEMPLATE_INVALID_NAME, "RECITE_SCHEMA001", "diagnostic-schema-001-availability-template-invalid-name", ["reason" => String, "name" => String]);
contract!(TEMPLATE_UNESCAPED_CLOSING_BRACE, "RECITE_SCHEMA001", "diagnostic-schema-001-availability-template-unescaped-closing-brace", ["reason" => String]);
contract!(TEMPLATE_UNKNOWN_PARAM, "RECITE_SCHEMA001", "diagnostic-schema-001-availability-template-unknown-param", ["reason" => String, "placeholder" => String]);
contract!(TEMPLATE_UNUSED_PARAM, "RECITE_SCHEMA001", "diagnostic-schema-001-availability-template-unused-param", ["reason" => String, "parameter" => String]);
contract!(ARGUMENT_DUPLICATE, "RECITE_SCHEMA003", "diagnostic-schema-003-availability-argument", ["condition" => String, "argument" => String]);
contract!(UNKNOWN_REASON_PARAM, "RECITE_SCHEMA001", "diagnostic-schema-001-availability-unknown-reason-param", ["condition" => String, "argument" => String]);
contract!(MISSING_REASON_ARG, "RECITE_SCHEMA001", "diagnostic-schema-001-availability-missing-reason-arg", ["condition" => String, "argument" => String]);
contract!(
    TAGGED_ONLY_TOML,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-availability-tagged-only-toml",
    []
);
contract!(
    TAG_MISSING_KIND,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-availability-tag-missing-kind",
    []
);
contract!(
    BINDING_MISSING_NAME,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-availability-binding-missing-name",
    []
);
contract!(
    LITERAL_MISSING_VALUE,
    "RECITE_SCHEMA001",
    "diagnostic-schema-001-availability-literal-missing-value",
    []
);
contract!(TAG_KIND, "RECITE_SCHEMA001", "diagnostic-schema-001-availability-tag-kind", ["kind" => String]);
contract!(BINDING_STRING_TYPE, "RECITE_SCHEMA001", "diagnostic-schema-001-availability-binding-string-type", ["condition" => String, "argument" => String, "expected" => String]);
contract!(BINDING_INT, "RECITE_SCHEMA001", "diagnostic-schema-001-availability-binding-int", ["condition" => String, "argument" => String]);
contract!(BINDING_LITERAL_TYPE, "RECITE_SCHEMA001", "diagnostic-schema-001-availability-binding-literal-type", ["condition" => String, "argument" => String, "expected" => String, "actual" => String]);
contract!(BINDING_UNKNOWN_VALUE, "RECITE_SCHEMA001", "diagnostic-schema-001-availability-binding-unknown-value", ["condition" => String, "argument" => String, "expected" => String, "value" => String]);
contract!(UNKNOWN_CONDITION_PARAM, "RECITE_SCHEMA004", "diagnostic-schema-004-unknown-condition-param", ["condition" => String, "condition_param" => String]);
contract!(BINDING_TYPE_MISMATCH, "RECITE_SCHEMA001", "diagnostic-schema-001-availability-binding-type-mismatch", ["condition" => String, "argument" => String, "expected" => String, "condition_param" => String, "actual" => String]);
contract!(DOMAIN_VALUES, "RECITE_SCHEMA001", "diagnostic-schema-001-domain-values", ["domain" => String]);
contract!(DOMAIN_MISSING_CONTEXT, "RECITE_SCHEMA001", "diagnostic-schema-001-domain-missing-context", ["domain" => String]);
contract!(DOMAIN_SELECTOR_REQUIRED, "RECITE_SCHEMA001", "diagnostic-schema-001-domain-selector-required", ["domain" => String]);
contract!(DOMAIN_SELECTOR, "RECITE_SCHEMA001", "diagnostic-schema-001-domain-selector", ["domain" => String, "selector" => String]);
contract!(DOMAIN_CONTEXT_VALUES, "RECITE_SCHEMA001", "diagnostic-schema-001-domain-context-values", ["domain" => String]);
contract!(DOMAIN_CONTEXT_DUPLICATE, "RECITE_SCHEMA003", "diagnostic-schema-003-domain-context", ["domain" => String, "context" => String]);
contract!(DOMAIN_KIND, "RECITE_SCHEMA001", "diagnostic-schema-001-domain-kind", ["domain" => String, "kind" => String]);
contract!(DOMAIN_POLICY_DOMAIN, "RECITE_SCHEMA001", "diagnostic-schema-001-domain-policy-domain", ["domain" => String, "policy" => String]);
contract!(DOMAIN_FALLBACK_DOMAIN, "RECITE_SCHEMA001", "diagnostic-schema-001-domain-fallback-domain", ["domain" => String]);
contract!(DOMAIN_POLICY, "RECITE_SCHEMA001", "diagnostic-schema-001-domain-policy", ["domain" => String, "policy" => String]);
contract!(DOMAIN_KIND_FIELD, "RECITE_SCHEMA001", "diagnostic-schema-001-domain-kind-field", ["domain" => String, "field" => String, "kind" => String]);
contract!(FLAT_VALUE_ORIGINS, "RECITE_SCHEMA001", "diagnostic-schema-001-flat-value-origins", ["owner" => String]);
contract!(CONTEXT_ORIGIN_NAME, "RECITE_SCHEMA001", "diagnostic-schema-001-context-origin-name", ["owner" => String]);
contract!(CONTEXTUAL_VALUE_ORIGINS, "RECITE_SCHEMA001", "diagnostic-schema-001-contextual-value-origins", ["owner" => String]);
contract!(CONTEXT_ORIGINS, "RECITE_SCHEMA001", "diagnostic-schema-001-context-origins", ["owner" => String]);

const CONTRACTS: &[&DiagnosticPresentationContract] = &[
    &NON_BOOL_MAPPING,
    &UNKNOWN_REASON,
    &TEMPLATE_UNTERMINATED,
    &TEMPLATE_INVALID_NAME,
    &TEMPLATE_UNESCAPED_CLOSING_BRACE,
    &TEMPLATE_UNKNOWN_PARAM,
    &TEMPLATE_UNUSED_PARAM,
    &ARGUMENT_DUPLICATE,
    &UNKNOWN_REASON_PARAM,
    &MISSING_REASON_ARG,
    &TAGGED_ONLY_TOML,
    &TAG_MISSING_KIND,
    &BINDING_MISSING_NAME,
    &LITERAL_MISSING_VALUE,
    &TAG_KIND,
    &BINDING_STRING_TYPE,
    &BINDING_INT,
    &BINDING_LITERAL_TYPE,
    &BINDING_UNKNOWN_VALUE,
    &UNKNOWN_CONDITION_PARAM,
    &BINDING_TYPE_MISMATCH,
    &DOMAIN_VALUES,
    &DOMAIN_MISSING_CONTEXT,
    &DOMAIN_SELECTOR_REQUIRED,
    &DOMAIN_SELECTOR,
    &DOMAIN_CONTEXT_VALUES,
    &DOMAIN_CONTEXT_DUPLICATE,
    &DOMAIN_KIND,
    &DOMAIN_POLICY_DOMAIN,
    &DOMAIN_FALLBACK_DOMAIN,
    &DOMAIN_POLICY,
    &DOMAIN_KIND_FIELD,
    &FLAT_VALUE_ORIGINS,
    &CONTEXT_ORIGIN_NAME,
    &CONTEXTUAL_VALUE_ORIGINS,
    &CONTEXT_ORIGINS,
];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}
