//! Validated, conflict-checked project manifest editing for authoring clients.
use crate::{ConfigWriteError, StateUpdateError, TextFileStore};
use recite_core::{Diagnostic, project::ProjectManifest as CoreManifest};
use std::{
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProjectSettingsError {
    #[error("Project manifest validation failed")]
    Validation(Vec<Diagnostic>),
    #[error(transparent)]
    Discovery(#[from] super::ProjectDiscoveryError),
    #[error(transparent)]
    Storage(#[from] ConfigWriteError),
    #[error("Could not read project schema {path}: {source}")]
    SchemaIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Project changes would remove an open document: {0}")]
    OpenDocument(PathBuf),
    #[error("The project manifest does not exist")]
    Missing,
    #[error("Project settings changed on disk. Reopen Settings to reload them.")]
    Conflict,
}

/// A project-owned manifest edit; user preferences never enter this file.
pub struct ProjectSettings {
    path: PathBuf,
    source: String,
}
impl ProjectSettings {
    pub fn open(root: &Path) -> Result<Self, ProjectSettingsError> {
        let report = super::discover_project(root)?;
        let path = report.manifest().manifest_path().to_owned();
        let source = TextFileStore::new(path.clone())
            .load()?
            .ok_or(ProjectSettingsError::Missing)?;
        Ok(Self { path, source })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn save(&mut self, replacement: &str) -> Result<(), ProjectSettingsError> {
        self.save_preserving_documents(replacement, &[])
    }

    /// Save only if every canonical document path remains in the prospective
    /// project. Authoring clients pass their active and retained sessions here.
    /// Validation runs under the same manifest lock as the conflict check.
    pub fn save_preserving_documents(
        &mut self,
        replacement: &str,
        documents: &[PathBuf],
    ) -> Result<(), ProjectSettingsError> {
        TextFileStore::new(self.path.clone())
            .update(|current| {
                if current != Some(self.source.as_str()) {
                    return Err(ProjectSettingsError::Conflict);
                }
                let report = validate(&self.path, replacement)?;
                for path in documents {
                    if !report
                        .documents()
                        .iter()
                        .any(|document| document.path() == path)
                    {
                        return Err(ProjectSettingsError::OpenDocument(path.clone()));
                    }
                }
                Ok(replacement.to_owned())
            })
            .map_err(|error| match error {
                StateUpdateError::Storage(error) => ProjectSettingsError::Storage(error),
                StateUpdateError::Edit(error) => error,
            })?;
        self.source = replacement.to_owned();
        Ok(())
    }
}

fn validate(
    path: &Path,
    text: &str,
) -> Result<super::ProjectDiscoveryReport, ProjectSettingsError> {
    let root = path.parent().ok_or(ProjectSettingsError::Missing)?;
    let report = super::manifest::discover_source(root.to_owned(), path.to_owned(), text)?;
    if !report.is_complete() {
        return Err(ProjectSettingsError::Validation(
            report
                .diagnostics()
                .iter()
                .map(|diagnostic| diagnostic.as_core_diagnostic())
                .collect(),
        ));
    }
    let loaded = CoreManifest::load_str_with_spans(path.to_string_lossy(), text);
    if !loaded.diagnostics.is_empty() {
        return Err(ProjectSettingsError::Validation(loaded.diagnostics));
    }
    let source = loaded.source.ok_or(ProjectSettingsError::Missing)?;
    let schema = if let Some(relative) = &source.manifest().project.schema {
        let root = path.parent().ok_or(ProjectSettingsError::Missing)?;
        let schema_path = root.join(relative);
        let text = std::fs::read_to_string(&schema_path).map_err(|source| {
            ProjectSettingsError::SchemaIo {
                path: schema_path,
                source,
            }
        })?;
        let loaded = recite_core::schema::load_schema_manifest_str(relative, &text);
        if !loaded.diagnostics.is_empty() {
            return Err(ProjectSettingsError::Validation(loaded.diagnostics));
        }
        loaded.schema
    } else {
        None
    };
    let diagnostics =
        recite_core::project::validate_project_manifest_source(&source, schema.as_ref());
    if !diagnostics.is_empty() {
        return Err(ProjectSettingsError::Validation(diagnostics));
    }
    Ok(report)
}
