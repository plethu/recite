use std::fs;

use recite_compiler::authoring::SavedDocument;
use recite_core::{project::validate_project_manifest_source, schema::load_schema_manifest_str};
use recite_writer_model::ProjectContext;

use crate::project::FileError;

pub(super) fn load(
    report: &recite_config::ProjectDiscoveryReport,
) -> Result<ProjectContext, FileError> {
    let manifest = report.manifest();
    let schema = if let Some(path) = &manifest.source().manifest().project.schema {
        let path = manifest.project_root().join(path);
        let text = fs::read_to_string(&path)?;
        let loaded = load_schema_manifest_str(path.to_string_lossy(), &text);
        if !loaded.diagnostics.is_empty() {
            return Err(FileError::Validation(loaded.diagnostics));
        }
        loaded.schema
    } else {
        None
    };
    let diagnostics = validate_project_manifest_source(manifest.source(), schema.as_ref());
    if !diagnostics.is_empty() {
        return Err(FileError::Validation(diagnostics));
    }
    Ok(ProjectContext {
        documents: report
            .documents()
            .iter()
            .map(|document| SavedDocument::new(document.key().clone(), document.text().to_owned()))
            .collect(),
        schema,
    })
}

pub(super) fn diagnostic_messages(diagnostics: &[recite_core::Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| {
            format!(
                "{}:{} · {} · {}",
                diagnostic.span.file,
                diagnostic.span.start.line(),
                diagnostic.code,
                diagnostic.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
