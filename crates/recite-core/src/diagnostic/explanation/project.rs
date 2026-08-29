use super::DiagnosticExplanation;
use crate::DiagnosticCategory;

pub(super) const EXPLANATIONS: &[DiagnosticExplanation] = &[
    DiagnosticExplanation::new(
        "RECITE_PROJECT001",
        DiagnosticCategory::Project,
        "The project manifest could not be parsed or does not match the manifest shape.",
        &["`recite.project.toml` is malformed or missing required fields."],
        &["Fix the manifest TOML and required project fields."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PROJECT002",
        DiagnosticCategory::Project,
        "Two scenes in the project manifest use the same scene ID.",
        &["A scene entry was copied without changing its key."],
        &["Give each scene a unique scene ID."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PROJECT003",
        DiagnosticCategory::Project,
        "A scene manifest references a compiled asset that is missing.",
        &["The asset has not been compiled or the manifest path is wrong."],
        &["Compile the asset or correct the manifest asset path."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PROJECT004",
        DiagnosticCategory::Project,
        "A scene manifest start block does not exist in the compiled asset.",
        &["The source block was renamed or the manifest points at the wrong block."],
        &["Update the manifest start block or recompile the intended source."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PROJECT005",
        DiagnosticCategory::Project,
        "A scene manifest is missing required participants.",
        &["The scene omits participant declarations needed by the project contract."],
        &["Add the required participant entries to the scene manifest."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PROJECT006",
        DiagnosticCategory::Project,
        "A scene manifest references a source asset that is missing.",
        &["The source file was moved, deleted, or the manifest path is wrong."],
        &["Restore the source file or correct the manifest source path."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PROJECT007",
        DiagnosticCategory::Project,
        "A referenced compiled asset is malformed, not a Recite asset, or uses an unsupported format.",
        &[
            "The file cannot be decoded as a valid Recite asset because it is malformed, from another format, or from an unsupported version.",
        ],
        &["Recompile the asset from source with the current Recite compiler."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PROJECT008",
        DiagnosticCategory::Project,
        "A project participant reference does not match the declared participant contract.",
        &[
            "The scene manifest omits the participant, or the compiled asset and schema participant sets differ.",
        ],
        &[
            "Declare the participant in the manifest and schema, or recompile the asset from the current participant contract.",
        ],
    ),
];
