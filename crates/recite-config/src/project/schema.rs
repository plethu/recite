use std::path::{Component, PathBuf};

use recite_core::{
    DocumentKey,
    schema::{SchemaLoadReport, load_schema_manifest_str},
};

use super::manifest::ProjectManifest;
use super::project_relative_key;

/// A schema filesystem failure, distinct from schema validation diagnostics.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProjectSchemaError {
    #[error("schema path {path} is not project-relative: {reason}")]
    InvalidPath { path: PathBuf, reason: String },
    #[error("schema path {declared} resolves outside the project to {resolved}")]
    OutsideProject {
        declared: PathBuf,
        resolved: PathBuf,
    },
    #[error("could not read project schema {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// The declared input identity and strict load report, including diagnostics.
pub struct LoadedProjectSchema {
    key: DocumentKey,
    path: PathBuf,
    report: SchemaLoadReport,
}

impl LoadedProjectSchema {
    pub fn key(&self) -> &DocumentKey {
        &self.key
    }
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
    pub fn into_report(self) -> SchemaLoadReport {
        self.report
    }
}

impl ProjectManifest {
    /// Resolve the declared schema identity without reading it. Internal
    /// symlink aliases retain their declared key, even when loading fails.
    pub fn schema_key(&self) -> Result<Option<DocumentKey>, ProjectSchemaError> {
        let Some(declared) = self.source().manifest().project.schema.as_deref() else {
            return Ok(None);
        };
        let path = self.project_root().join(declared);
        let invalid = |reason: &str| ProjectSchemaError::InvalidPath {
            path: path.clone(),
            reason: reason.to_owned(),
        };
        let relative = path
            .strip_prefix(self.project_root())
            .map_err(|_| invalid("path resolves outside the project"))?;
        for component in relative.components() {
            match component {
                Component::Normal(_) | Component::CurDir => {}
                Component::ParentDir => return Err(invalid("path contains a parent component")),
                Component::RootDir | Component::Prefix(_) => {
                    return Err(invalid("path is absolute"));
                }
            }
        }
        let key = project_relative_key(self.project_root(), &path)
            .ok_or_else(|| invalid("path is not UTF-8"))?;
        DocumentKey::new(key)
            .map(Some)
            .map_err(|error| invalid(&error.to_string()))
    }

    /// Load the schema declared by this source-backed manifest. Canonical
    /// targets must remain inside the project; internal symlinks are accepted.
    /// Diagnostics use the declared path, while filesystem failures identify
    /// the path that failed. No schema is distinct from an invalid schema.
    pub fn load_schema(&self) -> Result<Option<LoadedProjectSchema>, ProjectSchemaError> {
        let Some(key) = self.schema_key()? else {
            return Ok(None);
        };
        let declared = self.project_root().join(key.as_str());
        let path = std::fs::canonicalize(&declared).map_err(|source| ProjectSchemaError::Read {
            path: declared.clone(),
            source,
        })?;
        if !path.starts_with(self.project_root()) {
            return Err(ProjectSchemaError::OutsideProject {
                declared,
                resolved: path,
            });
        }
        let text = std::fs::read_to_string(&path).map_err(|source| ProjectSchemaError::Read {
            path: path.clone(),
            source,
        })?;
        let report = load_schema_manifest_str(declared.to_string_lossy().replace('\\', "/"), &text);
        Ok(Some(LoadedProjectSchema { key, path, report }))
    }
}
