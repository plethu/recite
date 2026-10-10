use std::collections::BTreeMap;
use std::path::Path;

use recite_compiler::authoring::{
    AuthoringKernel, AuthoringRequest, BuildGeneration, BuildInput, BuildInputAuthority,
    BuildInputKind, BuildRequest, SnapshotGeneration,
};
use recite_config::{ProjectDiscoveryError, ProjectDiscoveryReport, discover_project};
use recite_core::{Diagnostic, DiagnosticSeverity, DocumentKey, schema::ProjectSchema};

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
    prepare_discovered(discovery, generation, snapshot_generation)
}

pub fn prepare_discovered(
    discovery: ProjectDiscoveryReport,
    generation: BuildGeneration,
    snapshot_generation: SnapshotGeneration,
) -> Result<ProjectBuildPreparation, ProjectBuildPreparationError> {
    let discovered = discovery.manifest();
    let project_root = discovered.project_root().to_owned();
    let manifest = discovered.source().clone();
    let mut diagnostics = discovery
        .diagnostics()
        .iter()
        .map(recite_config::DiscoveryDiagnostic::as_core_diagnostic)
        .collect::<Vec<_>>();

    if !discovery.is_complete() {
        diagnostics.extend(
            validate_sources(discovery.documents(), None, false)
                .map_err(|message| ProjectBuildPreparationError::Authoring { message })?,
        );
        sort_diagnostics(&mut diagnostics);
        return Ok(ProjectBuildPreparation::Rejected { diagnostics });
    }

    let (schema, schema_key) = match discovered.load_schema().map_err(schema_failure)? {
        Some(loaded) => {
            let key = loaded.key().clone();
            let path = loaded.path().to_owned();
            let loaded = loaded.into_report();
            if !loaded.diagnostics.is_empty() {
                diagnostics.extend(loaded.diagnostics);
                sort_diagnostics(&mut diagnostics);
                return Ok(ProjectBuildPreparation::Rejected { diagnostics });
            }
            let schema = loaded
                .schema
                .ok_or(ProjectBuildPreparationError::SchemaWithoutModel { path })?;
            (Some(schema), Some(key))
        }
        None => (None, None),
    };

    diagnostics.extend(recite_core::project::validate_project_manifest_source(
        &manifest,
        schema.as_ref(),
    ));
    diagnostics.extend(
        validate_sources(discovery.documents(), schema.as_ref(), true)
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
        recite_compiler::authoring::BuildInputPolicy::SavedOnly,
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
    project_complete: bool,
) -> Result<Vec<Diagnostic>, String> {
    let mut kernel = match schema {
        Some(schema) => match AuthoringKernel::with_schema(schema.clone()) {
            Ok(kernel) => kernel,
            Err(recite_compiler::authoring::AuthoringError::InvalidSchema { diagnostics }) => {
                return Ok(diagnostics);
            }
            Err(error) => return Err(error.to_string()),
        },
        None => AuthoringKernel::new(),
    };
    let saved = documents.iter().map(|document| {
        recite_compiler::authoring::SavedDocument::new(document.key().clone(), document.text())
    });
    let request = AuthoringRequest::new(SnapshotGeneration::initial(), saved, std::iter::empty());
    if project_complete {
        kernel.apply(request)
    } else {
        kernel.apply_with_incomplete_project(request)
    }
    .map_err(|error| error.to_string())?;
    Ok(kernel.snapshot().diagnostics().iter().cloned().collect())
}

pub fn classify_discovery_error(
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

fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
}

fn schema_failure(error: recite_config::ProjectSchemaError) -> ProjectBuildPreparationError {
    match error {
        recite_config::ProjectSchemaError::InvalidPath { path, reason } => {
            ProjectBuildPreparationError::InvalidSchemaPath { path, reason }
        }
        recite_config::ProjectSchemaError::OutsideProject { declared, resolved } => {
            ProjectBuildPreparationError::SchemaOutsideProject { declared, resolved }
        }
        recite_config::ProjectSchemaError::Read { path, source } => {
            ProjectBuildPreparationError::Read {
                path,
                message: source.to_string(),
            }
        }
        _ => ProjectBuildPreparationError::Schema {
            message: error.to_string(),
        },
    }
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
