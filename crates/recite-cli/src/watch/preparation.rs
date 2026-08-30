use std::collections::BTreeMap;
use std::path::{Component, Path};

use recite_compiler::{
    AuthoringKernel, AuthoringRequest, BuildGeneration, BuildInput, BuildInputAuthority,
    BuildInputKind, BuildRequest, SnapshotGeneration,
};
use recite_config::{ProjectDiscoveryError, discover_project};
use recite_core::{Diagnostic, DiagnosticSeverity, DocumentKey, ProjectSchema};

use crate::fs::{load_schema, resolve_project_path};

use super::PROJECT_MANIFEST_FILE;
use super::request::{
    ProjectBuildPreparation, ProjectBuildPreparationError, ProjectBuildRequest, ProjectBuildTarget,
};

pub(super) fn prepare(
    project_root: &Path,
    generation: BuildGeneration,
    snapshot_generation: SnapshotGeneration,
) -> Result<ProjectBuildPreparation, ProjectBuildPreparationError> {
    let discovery = match discover_project(project_root) {
        Ok(report) => report,
        Err(error) => return classify_discovery_error(error),
    };
    let discovered = discovery.manifest();
    let project_root = discovered.project_root().to_owned();
    let manifest = discovered.source().clone();
    let mut diagnostics = discovery
        .diagnostics()
        .iter()
        .map(recite_config::DiscoveryDiagnostic::as_core_diagnostic)
        .collect::<Vec<_>>();

    if !discovery.is_complete() {
        sort_diagnostics(&mut diagnostics);
        return Ok(ProjectBuildPreparation::Rejected { diagnostics });
    }

    let (schema, schema_key) =
        match manifest.manifest().project.schema.as_deref() {
            Some(schema_path) => {
                let declared_path = resolve_project_path(&project_root, schema_path);
                let key = schema_document_key(&project_root, &declared_path).map_err(|reason| {
                    ProjectBuildPreparationError::InvalidSchemaPath {
                        path: declared_path.clone(),
                        reason,
                    }
                })?;
                let path = std::fs::canonicalize(&declared_path).map_err(|error| {
                    ProjectBuildPreparationError::Read {
                        path: declared_path.clone(),
                        message: error.to_string(),
                    }
                })?;
                if !path.starts_with(&project_root) {
                    return Err(ProjectBuildPreparationError::SchemaOutsideProject {
                        declared: declared_path,
                        resolved: path,
                    });
                }
                let loaded =
                    load_schema(&path).map_err(|error| ProjectBuildPreparationError::Read {
                        path: path.clone(),
                        message: error.to_string(),
                    })?;
                if !loaded.diagnostics.is_empty() {
                    diagnostics.extend(loaded.diagnostics);
                    sort_diagnostics(&mut diagnostics);
                    return Ok(ProjectBuildPreparation::Rejected { diagnostics });
                }
                let schema = loaded.schema.ok_or_else(|| {
                    ProjectBuildPreparationError::SchemaWithoutModel { path: path.clone() }
                })?;
                (Some(schema), Some(key))
            }
            None => (None, None),
        };

    diagnostics.extend(recite_core::project::validate_project_manifest_source(
        &manifest,
        schema.as_ref(),
    ));
    diagnostics.extend(
        validate_sources(discovery.documents(), schema.as_ref())
            .map_err(|message| ProjectBuildPreparationError::Authoring { message })?,
    );
    sort_diagnostics(&mut diagnostics);
    if has_errors(&diagnostics) {
        return Ok(ProjectBuildPreparation::Rejected { diagnostics });
    }

    if discovery.documents().is_empty() {
        return Err(ProjectBuildPreparationError::NoInputs);
    }

    let manifest_key = DocumentKey::new(PROJECT_MANIFEST_FILE.to_owned()).map_err(|error| {
        ProjectBuildPreparationError::InvalidInputKey {
            key: PROJECT_MANIFEST_FILE.to_owned(),
            reason: error.to_string(),
        }
    })?;
    let mut inputs = Vec::with_capacity(discovery.documents().len() + 2);
    inputs.push(BuildInput::new(
        manifest_key,
        BuildInputKind::Manifest,
        BuildInputAuthority::Saved,
        manifest.source_text(),
    ));
    for document in discovery.documents() {
        inputs.push(BuildInput::new(
            document.key().clone(),
            BuildInputKind::Source,
            BuildInputAuthority::Saved,
            document.text(),
        ));
    }
    if let (Some(schema), Some(key)) = (schema.clone(), schema_key) {
        inputs.push(BuildInput::schema(key, BuildInputAuthority::Saved, schema));
    }

    let build = BuildRequest::new_with_policy(
        generation,
        snapshot_generation,
        inputs,
        recite_compiler::BuildInputPolicy::SavedOnly,
    )?;
    let mut targets = BTreeMap::new();
    for scene in &manifest.manifest().scenes {
        let target = ProjectBuildTarget::new(&scene.asset)?;
        targets.entry(scene.asset.clone()).or_insert(target);
    }

    Ok(ProjectBuildPreparation::Ready(Box::new(
        ProjectBuildRequest {
            project_root,
            manifest,
            schema,
            build,
            targets: targets.into_values().collect(),
            diagnostics,
        },
    )))
}

fn validate_sources(
    documents: &[recite_config::DiscoveredDocument],
    schema: Option<&ProjectSchema>,
) -> Result<Vec<Diagnostic>, String> {
    let mut kernel = schema.map_or_else(AuthoringKernel::new, |schema| {
        AuthoringKernel::with_schema(schema.clone())
    });
    let saved = documents.iter().map(|document| {
        recite_compiler::SavedDocument::new(document.key().clone(), document.text())
    });
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            saved,
            std::iter::empty(),
        ))
        .map_err(|error| error.to_string())?;
    Ok(kernel.snapshot().diagnostics().iter().cloned().collect())
}

fn classify_discovery_error(
    error: ProjectDiscoveryError,
) -> Result<ProjectBuildPreparation, ProjectBuildPreparationError> {
    match error {
        ProjectDiscoveryError::NotFound { .. }
        | ProjectDiscoveryError::Read { .. }
        | ProjectDiscoveryError::NonUtf8 { .. } => {
            Err(ProjectBuildPreparationError::Discovery(error))
        }
        ProjectDiscoveryError::Malformed { .. }
        | ProjectDiscoveryError::MissingFormatVersion { .. }
        | ProjectDiscoveryError::UnsupportedFormatVersion { .. }
        | ProjectDiscoveryError::InvalidSourceRoot { .. }
        | ProjectDiscoveryError::InvalidExclude { .. }
        | ProjectDiscoveryError::DuplicateRoot { .. } => Ok(ProjectBuildPreparation::Rejected {
            diagnostics: error.diagnostics(),
        }),
        _ => Err(ProjectBuildPreparationError::Discovery(error)),
    }
}

fn schema_document_key(project_root: &Path, path: &Path) -> Result<DocumentKey, String> {
    let relative = if path.is_absolute() {
        path.strip_prefix(project_root)
            .map_err(|_| "path resolves outside the project".to_owned())?
    } else {
        path
    };
    let mut components = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(value) => components.push(value.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir => return Err("path contains a parent component".to_owned()),
            Component::RootDir | Component::Prefix(_) => {
                return Err("path is absolute".to_owned());
            }
        }
    }
    if components.is_empty() {
        return Err("path is empty".to_owned());
    }
    DocumentKey::new(components.join("/")).map_err(|error| error.to_string())
}

fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
}

fn sort_diagnostics(diagnostics: &mut [Diagnostic]) {
    diagnostics.sort_by(|left, right| {
        left.span
            .file
            .cmp(&right.span.file)
            .then(left.span.start.cmp(&right.span.start))
            .then(left.code.as_str().cmp(right.code.as_str()))
            .then(left.message.cmp(&right.message))
    });
}
