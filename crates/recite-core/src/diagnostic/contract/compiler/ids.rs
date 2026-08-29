use super::super::{
    DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticAuxiliaryPresentationContract,
    DiagnosticPresentationContract,
};

const NO_ARGUMENTS: &[DiagnosticArgumentSpec] = &[];
const ID: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "id",
    DiagnosticArgumentType::String,
)];

const MISSING_LINE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_ID001", "diagnostic-id-001", NO_ARGUMENTS);
const MISSING_CHOICE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_ID002", "diagnostic-id-002", NO_ARGUMENTS);
const DUPLICATE_LINE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_ID003", "diagnostic-id-003", ID);
const DUPLICATE_CHOICE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_ID004", "diagnostic-id-004", ID);
const DRAFT_LINE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_ID005", "diagnostic-id-005", NO_ARGUMENTS);
const DRAFT_CHOICE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_ID006", "diagnostic-id-006", NO_ARGUMENTS);
const MALFORMED_LINE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_ID007", "diagnostic-id-007", ID);
const MALFORMED_CHOICE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_ID008", "diagnostic-id-008", ID);

const MISSING_LINE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-id-001-help", NO_ARGUMENTS);
const MISSING_CHOICE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-id-002-help", NO_ARGUMENTS);
const DUPLICATE_LINE_RELATED: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-id-003-related", NO_ARGUMENTS);
const DUPLICATE_LINE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-id-003-help", NO_ARGUMENTS);
const DUPLICATE_CHOICE_RELATED: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-id-004-related", NO_ARGUMENTS);
const DUPLICATE_CHOICE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-id-004-help", NO_ARGUMENTS);
const DRAFT_LINE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-id-005-help", NO_ARGUMENTS);
const DRAFT_CHOICE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-id-006-help", NO_ARGUMENTS);
const MALFORMED_LINE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-id-007-help", NO_ARGUMENTS);
const MALFORMED_CHOICE_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-id-008-help", NO_ARGUMENTS);

static CONTRACTS: &[&DiagnosticPresentationContract] = &[
    &MISSING_LINE,
    &MISSING_CHOICE,
    &DUPLICATE_LINE,
    &DUPLICATE_CHOICE,
    &DRAFT_LINE,
    &DRAFT_CHOICE,
    &MALFORMED_LINE,
    &MALFORMED_CHOICE,
];
static AUXILIARY_CONTRACTS: &[&DiagnosticAuxiliaryPresentationContract] = &[
    &MISSING_LINE_HELP,
    &MISSING_CHOICE_HELP,
    &DUPLICATE_LINE_RELATED,
    &DUPLICATE_LINE_HELP,
    &DUPLICATE_CHOICE_RELATED,
    &DUPLICATE_CHOICE_HELP,
    &DRAFT_LINE_HELP,
    &DRAFT_CHOICE_HELP,
    &MALFORMED_LINE_HELP,
    &MALFORMED_CHOICE_HELP,
];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}

pub(super) fn auxiliary_contracts()
-> impl Iterator<Item = &'static DiagnosticAuxiliaryPresentationContract> {
    AUXILIARY_CONTRACTS.iter().copied()
}
