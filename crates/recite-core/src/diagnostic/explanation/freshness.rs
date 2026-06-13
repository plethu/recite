use super::DiagnosticExplanation;
use crate::DiagnosticCategory;

pub(super) const EXPLANATIONS: &[DiagnosticExplanation] = &[
    DiagnosticExplanation::new(
        "RECITE_FRESH001",
        DiagnosticCategory::Freshness,
        "A compiled asset was built from an older version of one or more source files.",
        &["The source dialogue changed after the asset was compiled."],
        &["Re-run `recite compile` or `recite watch` for the project."],
    ),
    DiagnosticExplanation::new(
        "RECITE_FRESH002",
        DiagnosticCategory::Freshness,
        "A compiled asset was built from an older schema fingerprint.",
        &["The schema manifest changed after the asset was compiled."],
        &["Recompile the asset with the current schema manifest."],
    ),
    DiagnosticExplanation::new(
        "RECITE_FRESH003",
        DiagnosticCategory::Freshness,
        "A compiled asset was produced by an incompatible compiler version or format.",
        &["The asset predates the supported compiler compatibility boundary."],
        &["Recompile the source with the current Recite compiler."],
    ),
];
