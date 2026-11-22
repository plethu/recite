use super::DiagnosticExplanation;
use crate::DiagnosticCategory;

pub(super) const EXPLANATIONS: &[DiagnosticExplanation] = &[
    DiagnosticExplanation::new(
        "RECITE_FRESH001",
        DiagnosticCategory::Freshness,
        "A compiled asset was built from an older version of the project's sources.",
        &["The source dialogue changed after the asset was compiled."],
        &["run `recite compile` again, or keep `recite watch .` running so it rebuilds on save"],
    ),
    DiagnosticExplanation::new(
        "RECITE_FRESH002",
        DiagnosticCategory::Freshness,
        "A compiled asset was built from an older schema fingerprint.",
        &["The schema manifest changed after the asset was compiled."],
        &["run `recite compile` again so the asset is checked against the current schema"],
    ),
    DiagnosticExplanation::new(
        "RECITE_FRESH003",
        DiagnosticCategory::Freshness,
        "A compiled asset was produced with a compiler compatibility version that this Recite version cannot use.",
        &[
            "The asset's compiler compatibility version is older or newer than the supported boundary.",
        ],
        &["run `recite compile` again with this version of Recite"],
    ),
    DiagnosticExplanation::new(
        "RECITE_FRESH004",
        DiagnosticCategory::Freshness,
        "A compiled asset does not include source files that were added to the project after it was built.",
        &["A `.recite` file was created inside the project's source roots after the last compile."],
        &["run `recite compile` again, or keep `recite watch .` running so it rebuilds on save"],
    ),
];
