use super::DiagnosticExplanation;
use crate::DiagnosticCategory;

pub(super) const EXPLANATIONS: &[DiagnosticExplanation] = &[
    DiagnosticExplanation::new(
        "RECITE_IMPORT001",
        DiagnosticCategory::Import,
        "Input or mapping errors prevent a usable migration result.",
        &[
            "The input is malformed, mapped fields are missing or ambiguous, or no supported blocks remain.",
        ],
        &["Correct the input or field mapping and inspect the report again."],
    ),
    DiagnosticExplanation::new(
        "RECITE_IMPORT002",
        DiagnosticCategory::Import,
        "A source construct was held back from generated Recite source.",
        &["The construct is outside the documented importer subset."],
        &[
            "Review its source provenance and migrate the construct manually before adopting the output.",
        ],
    ),
    DiagnosticExplanation::new(
        "RECITE_IMPORT003",
        DiagnosticCategory::Import,
        "A conversion may change source presentation or behavior.",
        &["Native source parsing may normalize the imported text's whitespace."],
        &["Compare the generated source with the original and review the change."],
    ),
];
