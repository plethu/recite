use recite_config::discover_project;
use recite_core::{Diagnostic, ProjectSchema, SchemaFingerprint};

use super::preparation::classify_discovery_error;
use super::{ProjectBuildPreparation, ProjectBuildPreparationError, ProjectBuildRequest};
use crate::error::CliError;
use crate::fs::{load_schema, resolve_project_path, validate_project_asset_freshness};

type CurrentSchema = (
    Option<ProjectSchema>,
    Option<SchemaFingerprint>,
    Vec<Diagnostic>,
);

pub(super) struct FreshnessResult {
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) stale: bool,
}

pub(super) fn assess_current_freshness(
    request: &ProjectBuildRequest,
) -> Result<FreshnessResult, CliError> {
    let report = match discover_project(request.project_root()) {
        Ok(report) => report,
        Err(error) => {
            return match classify_discovery_error(error) {
                Ok(ProjectBuildPreparation::Rejected { diagnostics }) => Ok(FreshnessResult {
                    diagnostics,
                    stale: false,
                }),
                Ok(ProjectBuildPreparation::Ready(_)) => Err(CliError::Watch {
                    message: "discovery error classification returned a ready request".to_owned(),
                }),
                Err(error) => Err(map_preparation_error(error)),
            };
        }
    };
    if !report.is_complete() {
        return Ok(FreshnessResult {
            diagnostics: report
                .diagnostics()
                .iter()
                .map(recite_config::DiscoveryDiagnostic::as_core_diagnostic)
                .collect(),
            stale: false,
        });
    }

    let manifest = report.manifest().source();
    let (schema, schema_fingerprint, mut diagnostics) =
        load_current_schema(report.manifest().project_root(), manifest)?;
    diagnostics.extend(recite_core::project::validate_project_manifest_source(
        manifest,
        schema.as_ref(),
    ));
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == recite_core::DiagnosticSeverity::Error)
    {
        return Ok(FreshnessResult {
            diagnostics,
            stale: false,
        });
    }
    diagnostics.extend(validate_project_asset_freshness(
        report.manifest().project_root(),
        manifest,
        schema_fingerprint,
    )?);
    let stale = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code.category() == recite_core::DiagnosticCategory::Freshness);
    Ok(FreshnessResult { diagnostics, stale })
}

fn load_current_schema(
    project_root: &std::path::Path,
    manifest: &recite_core::ProjectManifestSource,
) -> Result<CurrentSchema, CliError> {
    let Some(schema_path) = manifest.manifest().project.schema.as_deref() else {
        return Ok((None, Some(SchemaFingerprint::NoSchema), Vec::new()));
    };
    let schema_path = resolve_project_path(project_root, schema_path);
    let loaded = load_schema(&schema_path)?;
    if !loaded.diagnostics.is_empty() {
        return Ok((None, None, loaded.diagnostics));
    }
    let schema = loaded.schema.ok_or_else(|| CliError::Watch {
        message: format!(
            "schema input {} has no canonical model",
            schema_path.display()
        ),
    })?;
    let fingerprint = schema.canonical_fingerprint();
    Ok((Some(schema), Some(fingerprint), Vec::new()))
}

fn map_preparation_error(error: ProjectBuildPreparationError) -> CliError {
    match error {
        ProjectBuildPreparationError::Discovery(source) => CliError::ProjectDiscovery { source },
        ProjectBuildPreparationError::NoInputs => CliError::NoInputs,
        error => CliError::Watch {
            message: error.to_string(),
        },
    }
}
