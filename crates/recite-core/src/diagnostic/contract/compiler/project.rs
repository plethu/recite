use super::super::{
    DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticAuxiliaryPresentationContract,
    DiagnosticPresentationContract,
};

const NO_ARGUMENTS: &[DiagnosticArgumentSpec] = &[];
const BLOCK_ID: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "block_id",
    DiagnosticArgumentType::String,
)];
const REFERENCE: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "reference",
    DiagnosticArgumentType::String,
)];
const PATH: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "path",
    DiagnosticArgumentType::String,
)];

const MISSING_DEFAULT: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE005",
    "diagnostic-validate-005",
    NO_ARGUMENTS,
);
const AMBIGUOUS_DEFAULT: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE006", "diagnostic-validate-006", BLOCK_ID);
const UNKNOWN_REFERENCE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE007", "diagnostic-validate-007", REFERENCE);
const DUPLICATE_BLOCK: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE009", "diagnostic-validate-009", BLOCK_ID);
const DUPLICATE_PATH: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE010", "diagnostic-validate-010", PATH);
const AMBIGUOUS_COMPILED_BLOCK: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE011", "diagnostic-validate-011", BLOCK_ID);

const AMBIGUOUS_DEFAULT_RELATED: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-006-related", NO_ARGUMENTS);
const MISSING_DEFAULT_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-005-help", NO_ARGUMENTS);
const AMBIGUOUS_DEFAULT_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-006-help", NO_ARGUMENTS);
const DUPLICATE_BLOCK_RELATED: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-009-related", NO_ARGUMENTS);
const DUPLICATE_BLOCK_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-009-help", NO_ARGUMENTS);
const DUPLICATE_PATH_RELATED: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-010-related", NO_ARGUMENTS);
const DUPLICATE_PATH_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-010-help", NO_ARGUMENTS);
const AMBIGUOUS_COMPILED_RELATED: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-011-related", NO_ARGUMENTS);
const AMBIGUOUS_COMPILED_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-011-help", NO_ARGUMENTS);

static CONTRACTS: &[&DiagnosticPresentationContract] = &[
    &MISSING_DEFAULT,
    &AMBIGUOUS_DEFAULT,
    &UNKNOWN_REFERENCE,
    &DUPLICATE_BLOCK,
    &DUPLICATE_PATH,
    &AMBIGUOUS_COMPILED_BLOCK,
];
static AUXILIARY_CONTRACTS: &[&DiagnosticAuxiliaryPresentationContract] = &[
    &MISSING_DEFAULT_HELP,
    &AMBIGUOUS_DEFAULT_RELATED,
    &AMBIGUOUS_DEFAULT_HELP,
    &DUPLICATE_BLOCK_RELATED,
    &DUPLICATE_BLOCK_HELP,
    &DUPLICATE_PATH_RELATED,
    &DUPLICATE_PATH_HELP,
    &AMBIGUOUS_COMPILED_RELATED,
    &AMBIGUOUS_COMPILED_HELP,
];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}

pub(super) fn auxiliary_contracts()
-> impl Iterator<Item = &'static DiagnosticAuxiliaryPresentationContract> {
    AUXILIARY_CONTRACTS.iter().copied()
}
