use super::DiagnosticExplanation;
use crate::DiagnosticCategory;

pub(super) const EXPLANATIONS: &[DiagnosticExplanation] = &[
    DiagnosticExplanation::new(
        "RECITE_ID001",
        DiagnosticCategory::Identifier,
        "A dialogue line is missing its required stable line ID.",
        &["A line was written without a trailing `@id`."],
        &["Add a stable line ID and keep it unchanged once authored."],
    ),
    DiagnosticExplanation::new(
        "RECITE_ID002",
        DiagnosticCategory::Identifier,
        "A choice is missing its required stable choice ID.",
        &["A choice was written without a trailing `@id`."],
        &["Add a stable choice ID and keep it unchanged once authored."],
    ),
    DiagnosticExplanation::new(
        "RECITE_ID003",
        DiagnosticCategory::Identifier,
        "Two or more lines use the same stable line ID.",
        &["A line was copied without changing the ID."],
        &["Give each distinct line a unique stable line ID."],
    ),
    DiagnosticExplanation::new(
        "RECITE_ID004",
        DiagnosticCategory::Identifier,
        "Two or more choices use the same stable choice ID.",
        &["A choice was copied without changing the ID."],
        &["Give each distinct choice a unique stable choice ID."],
    ),
    DiagnosticExplanation::new(
        "RECITE_ID007",
        DiagnosticCategory::Identifier,
        "A line ID has an invalid shape.",
        &["The ID contains unsupported characters or does not match the stable ID format."],
        &["Rename the line ID to the supported stable ID shape."],
    ),
    DiagnosticExplanation::new(
        "RECITE_ID008",
        DiagnosticCategory::Identifier,
        "A choice ID has an invalid shape.",
        &["The ID contains unsupported characters or does not match the stable ID format."],
        &["Rename the choice ID to the supported stable ID shape."],
    ),
    DiagnosticExplanation::new(
        "RECITE_ID034",
        DiagnosticCategory::Identifier,
        "A PO catalogue stable ID is malformed.",
        &["A PO context is not a valid Recite stable ID."],
        &["Restore the stable ID that the source header uses."],
    ),
    DiagnosticExplanation::new(
        "RECITE_ID035",
        DiagnosticCategory::Identifier,
        "A PO catalogue contains duplicate active durable keys.",
        &["Two active entries use the same stable context and source text."],
        &[
            "Keep one active entry per stable catalogue key; retain stale history as fuzzy or obsolete.",
        ],
    ),
];
