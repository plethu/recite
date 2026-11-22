//! A speaker the schema does not declare is kept as written and reported once for review, so one
//! unmapped character name does not reject an otherwise valid import.
use std::collections::BTreeSet;

use recite_compiler::compile::{
    CompileInput, CompileOptions, ModelCompileReport, compile_inputs_to_model,
};
use recite_core::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticCode, DiagnosticSeverity, schema::ProjectSchema,
};

use crate::{Action, ImportError, ImportItem, Location, Provenance, SourceMapping};

const UNKNOWN_SPEAKER: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE051");

/// The generated source whose native validation is reported.
pub(super) const GENERATED: &str = "imported.recite";

/// Compile the generated source. When speakers missing from the schema are its only errors,
/// compile again without the speaker registry, so everything else is still validated, and return
/// each missing speaker once as a review item at its first use in the input.
pub(super) fn compile(
    inputs: Vec<CompileInput>,
    options: CompileOptions,
    schema: Option<&ProjectSchema>,
    mappings: &[SourceMapping],
    file: &str,
) -> Result<(ModelCompileReport, Vec<ImportItem>), ImportError> {
    let report = compile_inputs_to_model(inputs.clone(), options.clone(), schema)?;
    let Some(schema) = schema else {
        return Ok((report, Vec::new()));
    };
    let mut errors = report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .peekable();
    if errors.peek().is_none() || !errors.all(|diagnostic| diagnostic.code == UNKNOWN_SPEAKER) {
        return Ok((report, Vec::new()));
    }
    let mut reported = BTreeSet::new();
    let items = report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == UNKNOWN_SPEAKER)
        .filter(|diagnostic| reported.insert(speaker(diagnostic)))
        .map(|diagnostic| item(diagnostic, mappings, file))
        .collect::<Result<Vec<_>, _>>()?;
    let mut open = schema.clone();
    open.speakers.clear();
    let report = compile_inputs_to_model(inputs, options, Some(&open))?;
    Ok((report, items))
}

fn speaker(diagnostic: &Diagnostic) -> String {
    match diagnostic
        .presentation
        .as_ref()
        .and_then(|presentation| presentation.arguments().get("speaker"))
    {
        Some(DiagnosticArgumentValue::String(speaker)) => speaker.clone(),
        _ => diagnostic.message.clone(),
    }
}

fn item(
    diagnostic: &Diagnostic,
    mappings: &[SourceMapping],
    file: &str,
) -> Result<ImportItem, ImportError> {
    let line = usize::try_from(diagnostic.span.start.line()).unwrap_or(usize::MAX);
    let provenance = mappings
        .iter()
        .find(|mapping| diagnostic.span.file == GENERATED && mapping.generated_line == line)
        .map_or_else(
            || Provenance {
                file: file.to_owned(),
                location: Location::Document,
                record_key: "document".to_owned(),
            },
            |mapping| mapping.provenance.clone(),
        );
    Ok(ImportItem {
        diagnostic: diagnostic.record()?,
        provenance,
        construct: "speaker".to_owned(),
        action: Action::Unresolved,
        follow_up: "Declare this speaker in the schema, or correct the name in the input, then \
                    import again."
            .to_owned(),
    })
}
