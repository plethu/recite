use super::super::{
    DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticAuxiliaryPresentationContract,
    DiagnosticPresentationContract,
};

const NO_ARGUMENTS: &[DiagnosticArgumentSpec] = &[];
const PATTERN: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "pattern",
    DiagnosticArgumentType::String,
)];

const VARIANT: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("pattern", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("enum", DiagnosticArgumentType::String),
];
const MISSING: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("enum", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("missing", DiagnosticArgumentType::String),
];
const SUGGESTION: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "suggestion",
    DiagnosticArgumentType::String,
)];

const DUPLICATE_CASE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE052", "diagnostic-validate-052", PATTERN);
const CASE_AFTER_WILDCARD: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE053", "diagnostic-validate-053", PATTERN);
const UNKNOWN_VARIANT: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE054", "diagnostic-validate-054", VARIANT);
const NON_EXHAUSTIVE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE055", "diagnostic-validate-055", MISSING);

const DUPLICATE_CASE_RELATED: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-052-related", NO_ARGUMENTS);
const DUPLICATE_CASE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-052-help", NO_ARGUMENTS);
const CASE_AFTER_WILDCARD_RELATED: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-053-related", NO_ARGUMENTS);
const CASE_AFTER_WILDCARD_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-053-help", NO_ARGUMENTS);
const UNKNOWN_VARIANT_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-054-help", NO_ARGUMENTS);
const UNKNOWN_VARIANT_SUGGESTION: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new(
        "diagnostic-validate-054-help-suggestion",
        SUGGESTION,
    );
const NON_EXHAUSTIVE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-055-help", NO_ARGUMENTS);

static CONTRACTS: &[&DiagnosticPresentationContract] = &[
    &DUPLICATE_CASE,
    &CASE_AFTER_WILDCARD,
    &UNKNOWN_VARIANT,
    &NON_EXHAUSTIVE,
];
static AUXILIARY_CONTRACTS: &[&DiagnosticAuxiliaryPresentationContract] = &[
    &DUPLICATE_CASE_RELATED,
    &DUPLICATE_CASE_HELP,
    &CASE_AFTER_WILDCARD_RELATED,
    &CASE_AFTER_WILDCARD_HELP,
    &UNKNOWN_VARIANT_HELP,
    &UNKNOWN_VARIANT_SUGGESTION,
    &NON_EXHAUSTIVE_HELP,
];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}

pub(super) fn auxiliary_contracts()
-> impl Iterator<Item = &'static DiagnosticAuxiliaryPresentationContract> {
    AUXILIARY_CONTRACTS.iter().copied()
}
