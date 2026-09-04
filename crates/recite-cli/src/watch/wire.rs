use std::path::Path;

use recite_compiler::BuildStatusProjection;

use crate::structured::data::diagnostic_records;
use crate::structured::errors::StructuredError;

use super::ProjectBuildRecovery;
use super::build::BuildStatus;
use super::wire_types::*;

#[path = "wire_mapping.rs"]
mod mapping;

use mapping::{
    artifact_metadata_for_publication, build_outcome, build_status, cancellation, failure,
    freshness, host_policy_required, input_keys, outcome_dto, publication, recovery_record,
    restart_guidance, status_dto,
};

#[cfg(test)]
mod tests;

impl BuildCompletedData {
    pub(super) fn from_projection(
        projection: &BuildStatusProjection,
        project_root: &Path,
        status: &BuildStatus,
        recovery: &[ProjectBuildRecovery],
    ) -> Result<Self, crate::error::CliError> {
        Self::from_projection_parts(
            projection,
            project_root,
            status_dto(status),
            outcome_dto(status, projection),
            recovery,
            None,
        )
    }

    pub(super) fn from_projection_error(
        projection: &BuildStatusProjection,
        project_root: &Path,
        recovery: &[ProjectBuildRecovery],
        error: StructuredError,
    ) -> Result<Self, crate::error::CliError> {
        Self::from_projection_parts(
            projection,
            project_root,
            projection
                .terminal_status()
                .map_or(BuildStatusDto::Failed, build_status),
            build_outcome(projection),
            recovery,
            Some(error),
        )
    }

    fn from_projection_parts(
        projection: &BuildStatusProjection,
        project_root: &Path,
        status: BuildStatusDto,
        outcome: BuildOutcomeDto,
        recovery: &[ProjectBuildRecovery],
        error: Option<StructuredError>,
    ) -> Result<Self, crate::error::CliError> {
        let diagnostics = diagnostic_records(projection.diagnostics())?;
        let publication = projection.publish().map_or(
            PublicationDto::NotAttempted {
                reason: PublishNotAttemptedReasonDto::BuildFailed,
            },
            publication,
        );
        let artifacts = artifact_metadata_for_publication(
            project_root,
            projection.publish(),
            projection.candidates(),
        )?;
        let mut recovery_records = recovery.to_vec();
        recovery_records.sort();
        recovery_records.dedup();
        Ok(Self {
            generation: projection.generation().map_or(0, |value| value.as_u64()),
            snapshot_generation: projection.snapshot_generation().map(|value| value.as_u64()),
            status,
            outcome,
            inputs: input_keys(projection),
            diagnostics,
            artifacts,
            freshness: projection
                .freshness()
                .map_or(FreshnessDto::Unknown, freshness),
            publication,
            recovery: recovery_records.iter().map(recovery_record).collect(),
            restart_guidance: restart_guidance(projection.restart_guidance()),
            cancellation: projection.cancellation().map(cancellation),
            failure: projection.failure().map(failure),
            error,
        })
    }

    pub(super) fn from_diagnostics(
        generation: u64,
        inputs: &[String],
        diagnostics: &[recite_core::Diagnostic],
    ) -> Result<Self, crate::error::CliError> {
        Ok(Self {
            generation,
            snapshot_generation: None,
            status: BuildStatusDto::Failed,
            outcome: BuildOutcomeDto::Diagnostics,
            inputs: sorted_inputs(inputs),
            diagnostics: diagnostic_records(diagnostics)?,
            artifacts: Vec::new(),
            freshness: FreshnessDto::Unknown,
            publication: PublicationDto::NotAttempted {
                reason: PublishNotAttemptedReasonDto::PreparationFailed,
            },
            recovery: Vec::new(),
            restart_guidance: host_policy_required(),
            cancellation: None,
            failure: Some(FailureDto::Diagnostics),
            error: None,
        })
    }

    pub(super) fn from_error(generation: u64, inputs: &[String], error: StructuredError) -> Self {
        Self {
            generation,
            snapshot_generation: None,
            status: BuildStatusDto::Failed,
            outcome: BuildOutcomeDto::OperationalFailure,
            inputs: sorted_inputs(inputs),
            diagnostics: Vec::new(),
            artifacts: Vec::new(),
            freshness: FreshnessDto::Unknown,
            publication: PublicationDto::NotAttempted {
                reason: PublishNotAttemptedReasonDto::PreparationFailed,
            },
            recovery: Vec::new(),
            restart_guidance: host_policy_required(),
            cancellation: None,
            failure: None,
            error: Some(error),
        }
    }
}

fn sorted_inputs(inputs: &[String]) -> Vec<String> {
    let mut inputs = inputs.to_vec();
    inputs.sort();
    inputs.dedup();
    inputs
}
