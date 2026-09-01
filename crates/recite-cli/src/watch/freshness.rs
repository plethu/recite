use recite_compiler::{FreshnessAssessment, FreshnessStatus, StaleReason};
use recite_core::{Diagnostic, SchemaFingerprint};

use super::{ProjectBuildPreparation, ProjectBuildPreparationError, ProjectBuildRequest};
use crate::error::CliError;
use crate::fs::validate_project_asset_freshness;

pub(super) struct FreshnessResult {
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) assessment: FreshnessAssessment,
}

pub(super) fn assess_current_freshness(
    request: &ProjectBuildRequest,
) -> Result<FreshnessResult, CliError> {
    let current = ProjectBuildRequest::prepare_with_generations(
        request.project_root(),
        request.build_request().generation(),
        request.build_request().snapshot_generation(),
    )
    .map_err(map_preparation_error)?;
    let current = match current {
        ProjectBuildPreparation::Ready(request) => *request,
        ProjectBuildPreparation::Rejected { diagnostics } => {
            return Ok(FreshnessResult {
                diagnostics,
                // Publication completed from the previous valid request, but
                // the recheck could not prepare the current project. The
                // published asset is therefore known not to describe the
                // current project, even though no fingerprint comparison was
                // possible.
                assessment: FreshnessAssessment::stale(
                    request.build_request().fingerprints().clone(),
                    vec![StaleReason::Fingerprints],
                ),
            });
        }
    };

    if !same_published_request(request, &current) {
        return Ok(FreshnessResult {
            diagnostics: Vec::new(),
            assessment: FreshnessAssessment::stale(
                request.build_request().fingerprints().clone(),
                vec![StaleReason::Fingerprints],
            ),
        });
    }

    let schema_fingerprint = current
        .schema()
        .map_or(SchemaFingerprint::NoSchema, |schema| {
            schema.canonical_fingerprint()
        });
    let diagnostics = validate_project_asset_freshness(
        current.project_root(),
        current.manifest(),
        Some(schema_fingerprint),
    )?;
    let stale = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code.category() == recite_core::DiagnosticCategory::Freshness);
    let assessment = if stale {
        FreshnessAssessment::stale(
            request.build_request().fingerprints().clone(),
            vec![StaleReason::Fingerprints],
        )
    } else {
        FreshnessAssessment::fresh(request.build_request().fingerprints().clone())
    };
    debug_assert!(matches!(
        assessment.status(),
        FreshnessStatus::Fresh | FreshnessStatus::Stale
    ));
    Ok(FreshnessResult {
        diagnostics,
        assessment,
    })
}

fn same_published_request(published: &ProjectBuildRequest, current: &ProjectBuildRequest) -> bool {
    published.project_root() == current.project_root()
        && published.build_request() == current.build_request()
        && published.targets() == current.targets()
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
