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
        "A referenced compiled asset could not be decoded as Recite dialogue.",
        &["The file is not a Recite asset or it was produced with an unsupported format."],
        &["Recompile the asset from source and verify the manifest path."],
    ),
    DiagnosticExplanation::new(
        "RECITE_PROJECT008",
        DiagnosticCategory::Project,
        "A project scene references a participant that is not declared.",
        &["The participant name is misspelled or missing from the project manifest."],
        &["Declare the participant or update the scene reference."],
    ),
];
