use std::path::PathBuf;

use recite_compiler::authoring::{
    AuthoringKernel, AuthoringRequest, SavedDocument, SnapshotGeneration,
};
use recite_config::discover_project;
use recite_core::{
    Diagnostic,
    compiled::SchemaFingerprint,
    project::validate_project_manifest_source,
    schema::{ProjectSchema, SchemaLoadReport},
};

use super::project_freshness::validate_project_asset_freshness;
use crate::error::CliError;

pub(crate) fn validate_project(project_root: PathBuf) -> Result<Vec<Diagnostic>, CliError> {
    validate_project_with_mode(project_root, ProjectValidationMode::Authoring)
}

pub(crate) fn check_fresh(project_root: PathBuf) -> Result<Vec<Diagnostic>, CliError> {
    validate_project_with_mode(project_root, ProjectValidationMode::FreshnessOnly)
}

#[derive(Clone, Copy)]
enum ProjectValidationMode {
    Authoring,
    FreshnessOnly,
}

fn validate_project_with_mode(
    project_root: PathBuf,
    mode: ProjectValidationMode,
) -> Result<Vec<Diagnostic>, CliError> {
    let report = match discover_project(&project_root) {
        Ok(report) => report,
        Err(recite_config::ProjectDiscoveryError::Malformed { diagnostics, .. }) => {
            return Ok(diagnostics);
        }
        Err(source) => return Ok(vec![source.as_core_diagnostic()]),
    };
    let discovered = report.manifest();
    let project_root = discovered.project_root().to_owned();
    let manifest_source = discovered.source();
    let mut diagnostics = report
        .diagnostics()
        .iter()
        .map(recite_config::DiscoveryDiagnostic::as_core_diagnostic)
        .collect::<Vec<_>>();

    let loaded_schema = match discovered
        .load_schema()
        .map_err(|source| CliError::ProjectSchema { source })?
    {
        Some(loaded) => loaded.into_report(),
        None => SchemaLoadReport {
            schema: None,
            diagnostics: Vec::new(),
        },
    };
    diagnostics.extend(loaded_schema.diagnostics.iter().cloned());
    diagnostics.extend(validate_project_manifest_source(
        manifest_source,
        loaded_schema.schema.as_ref(),
    ));

    if matches!(mode, ProjectValidationMode::Authoring) {
        diagnostics.extend(validate_project_sources(
            report.documents(),
            loaded_schema.schema.as_ref(),
            loaded_schema.diagnostics.is_empty(),
            report.is_complete(),
        )?);
    }

    diagnostics.extend(validate_project_asset_freshness(
        &project_root,
        manifest_source,
        match (
            loaded_schema.schema.as_ref(),
            loaded_schema.diagnostics.is_empty(),
        ) {
            (Some(schema), true) => Some(ProjectSchema::canonical_fingerprint(schema)),
            (None, true) => Some(SchemaFingerprint::NoSchema),
            (_, false) => None,
        },
    )?);

    Ok(diagnostics)
}

fn validate_project_sources(
    documents: &[recite_config::DiscoveredDocument],
    schema: Option<&ProjectSchema>,
    schema_is_valid: bool,
    project_complete: bool,
) -> Result<Vec<Diagnostic>, CliError> {
    let saved_documents = documents
        .iter()
        .map(|document| SavedDocument::new(document.key().clone(), document.text().to_owned()));
    let mut kernel = match (schema, schema_is_valid) {
        (Some(schema), true) => match AuthoringKernel::with_schema(schema.clone()) {
            Ok(kernel) => kernel,
            Err(recite_compiler::authoring::AuthoringError::InvalidSchema { diagnostics }) => {
                return Ok(diagnostics);
            }
            Err(error) => {
                return Err(CliError::Compile(
                    recite_compiler::compile::CompileError::InvalidValidatedInput(
                        error.to_string(),
                    ),
                ));
            }
        },
        _ => AuthoringKernel::new(),
    };
    let request = AuthoringRequest::new(
        SnapshotGeneration::initial(),
        saved_documents,
        std::iter::empty(),
    );
    if project_complete {
        kernel.apply(request)
    } else {
        kernel.apply_with_incomplete_project(request)
    }
    .map_err(|error| {
        CliError::Compile(
            recite_compiler::compile::CompileError::InvalidValidatedInput(format!(
                "authoring kernel rejected initial project request: {error}"
            )),
        )
    })?;

    Ok(kernel.snapshot().diagnostics().iter().cloned().collect())
}
