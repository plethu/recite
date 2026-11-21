use super::{DiagnosticArgumentSpec, DiagnosticArgumentType, DiagnosticPresentationContract};

/// Findings name the flagged text, the checker rule and its English detail.
const FINDING: &[DiagnosticArgumentSpec] = &[
    DiagnosticArgumentSpec::new("text", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("rule", DiagnosticArgumentType::String),
    DiagnosticArgumentSpec::new("detail", DiagnosticArgumentType::String),
];
const DETAIL: &[DiagnosticArgumentSpec] = &[DiagnosticArgumentSpec::new(
    "detail",
    DiagnosticArgumentType::String,
)];

const SPELLING: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PROSE001", "diagnostic-prose-001", FINDING);
const GRAMMAR: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PROSE002", "diagnostic-prose-002", FINDING);
const STYLE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PROSE003", "diagnostic-prose-003", FINDING);
const LANGUAGE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PROSE101", "diagnostic-prose-101", DETAIL);
const DICTIONARY: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PROSE102", "diagnostic-prose-102", DETAIL);
const ENGINE: DiagnosticPresentationContract =
    DiagnosticPresentationContract::new("RECITE_PROSE103", "diagnostic-prose-103", DETAIL);

static CONTRACTS: &[&DiagnosticPresentationContract] =
    &[&SPELLING, &GRAMMAR, &STYLE, &LANGUAGE, &DICTIONARY, &ENGINE];

pub(super) fn contracts() -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    CONTRACTS.iter().copied()
}
