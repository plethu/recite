use super::DiagnosticExplanation;
use crate::DiagnosticCategory;

pub(super) const EXPLANATIONS: &[DiagnosticExplanation] = &[
    DiagnosticExplanation::new(
        "RECITE_ID001",
        DiagnosticCategory::Identifier,
        "A dialogue line is missing its required stable line ID.",
        &["A line was written without an `@id` anchor."],
        &["Add a stable line ID and keep it unchanged once authored."],
    ),
    DiagnosticExplanation::new(
        "RECITE_ID002",
        DiagnosticCategory::Identifier,
        "A choice is missing its required stable choice ID.",
        &["A choice was written without an `@id` anchor."],
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
        "RECITE_ID005",
        DiagnosticCategory::Identifier,
        "A line still uses a draft line ID.",
        &["The line was generated or stubbed and was never assigned a final ID."],
        &["Replace the draft ID with the final stable line ID before shipping."],
    ),
    DiagnosticExplanation::new(
        "RECITE_ID006",
        DiagnosticCategory::Identifier,
        "A choice still uses a draft choice ID.",
        &["The choice was generated or stubbed and was never assigned a final ID."],
        &["Replace the draft ID with the final stable choice ID before shipping."],
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
];
