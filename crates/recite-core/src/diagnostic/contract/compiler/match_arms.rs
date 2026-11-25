use super::super::{
    DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticAuxiliaryPresentationContract,
    DiagnosticPresentationContract,
};

const NO_ARGUMENTS: &[DiagnosticArgumentSpec] = &[];
const PATTERN: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "pattern",
    DiagnosticArgumentType::String,
)];

const DUPLICATE_CASE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE052", "diagnostic-validate-052", PATTERN);
const CASE_AFTER_WILDCARD: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE053", "diagnostic-validate-053", PATTERN);

const DUPLICATE_CASE_RELATED: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-052-related", NO_ARGUMENTS);
const DUPLICATE_CASE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-052-help", NO_ARGUMENTS);
const CASE_AFTER_WILDCARD_RELATED: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-053-related", NO_ARGUMENTS);
const CASE_AFTER_WILDCARD_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-053-help", NO_ARGUMENTS);

static CONTRACTS: &[&DiagnosticPresentationContract] = &[&DUPLICATE_CASE, &CASE_AFTER_WILDCARD];
static AUXILIARY_CONTRACTS: &[&DiagnosticAuxiliaryPresentationContract] = &[
    &DUPLICATE_CASE_RELATED,
    &DUPLICATE_CASE_HELP,
    &CASE_AFTER_WILDCARD_RELATED,
    &CASE_AFTER_WILDCARD_HELP,
];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}

pub(super) fn auxiliary_contracts()
-> impl Iterator<Item = &'static DiagnosticAuxiliaryPresentationContract> {
    AUXILIARY_CONTRACTS.iter().copied()
}
