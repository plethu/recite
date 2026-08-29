use super::super::{
    DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticAuxiliaryPresentationContract,
    DiagnosticPresentationContract,
};

const NO_ARGUMENTS: &[DiagnosticArgumentSpec] = &[];
const TAG: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "tag",
    DiagnosticArgumentType::String,
)];
const TAGS: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("tag", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("expected_tag", DiagnosticArgumentType::String),
];
const PARENT_CHILD: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("parent", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("child", DiagnosticArgumentType::String),
];

const UNKNOWN_TAG: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE022", "diagnostic-validate-022", TAG);
const UNBALANCED_BRACKET: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE023",
    "diagnostic-validate-023-bracket",
    NO_ARGUMENTS,
);
const UNBALANCED_STANDALONE: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE023",
    "diagnostic-validate-023-standalone",
    TAG,
);
const UNBALANCED_NO_OPENING: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE023",
    "diagnostic-validate-023-no-opening",
    TAG,
);
const UNBALANCED_MISMATCH: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE023",
    "diagnostic-validate-023-mismatch",
    TAGS,
);
const MISSING_CLOSING: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_VALIDATE024", "diagnostic-validate-024", TAG);
const INVALID_NESTING: DiagnosticPresentationContract = DiagnosticPresentationContract::new(
    "RECITE_VALIDATE025",
    "diagnostic-validate-025",
    PARENT_CHILD,
);

const UNKNOWN_TAG_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-022-help", NO_ARGUMENTS);
const UNBALANCED_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-023-help", NO_ARGUMENTS);
const UNBALANCED_RELATED: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-023-related", NO_ARGUMENTS);
const MISSING_CLOSING_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-024-help", TAG);
const INVALID_NESTING_RELATED: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-025-related", NO_ARGUMENTS);
const INVALID_NESTING_HELP: DiagnosticAuxiliaryPresentationContract =
    DiagnosticAuxiliaryPresentationContract::new("diagnostic-validate-025-help", PARENT_CHILD);

static CONTRACTS: &[&DiagnosticPresentationContract] = &[
    &UNKNOWN_TAG,
    &UNBALANCED_BRACKET,
    &UNBALANCED_STANDALONE,
    &UNBALANCED_NO_OPENING,
    &UNBALANCED_MISMATCH,
    &MISSING_CLOSING,
    &INVALID_NESTING,
];
static AUXILIARY_CONTRACTS: &[&DiagnosticAuxiliaryPresentationContract] = &[
    &UNKNOWN_TAG_HELP,
    &UNBALANCED_HELP,
    &UNBALANCED_RELATED,
    &MISSING_CLOSING_HELP,
    &INVALID_NESTING_RELATED,
    &INVALID_NESTING_HELP,
];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}

pub(super) fn auxiliary_contracts()
-> impl Iterator<Item = &'static DiagnosticAuxiliaryPresentationContract> {
    AUXILIARY_CONTRACTS.iter().copied()
}
