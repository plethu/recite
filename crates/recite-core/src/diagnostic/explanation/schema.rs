use super::DiagnosticExplanation;
use crate::DiagnosticCategory;

pub(super) const EXPLANATIONS: &[DiagnosticExplanation] = &[
    DiagnosticExplanation::new(
        "RECITE_SCHEMA001",
        DiagnosticCategory::Schema,
        "A schema manifest has a malformed shape.",
        &["The JSON is valid enough to read but does not match the Recite schema model."],
        &["Fix the schema field shape reported by the diagnostic."],
    ),
    DiagnosticExplanation::new(
        "RECITE_SCHEMA002",
        DiagnosticCategory::Schema,
        "A schema manifest declares an unsupported schema version.",
        &["The manifest was produced for a newer or incompatible Recite schema version."],
        &["Use a supported schema version or regenerate the manifest with this Recite version."],
    ),
    DiagnosticExplanation::new(
        "RECITE_SCHEMA003",
        DiagnosticCategory::Schema,
        "A schema manifest defines the same item more than once.",
        &["A type, speaker, condition, effect, registry, domain, or reason name is duplicated."],
        &["Rename or remove the duplicate schema definition."],
    ),
    DiagnosticExplanation::new(
        "RECITE_SCHEMA004",
        DiagnosticCategory::Schema,
        "A schema manifest references an unknown or invalid type.",
        &[
            "A field, parameter, return value, or availability argument names a type that is not defined.",
        ],
        &["Define the type or update the reference to an existing schema type."],
    ),
];
