use std::path::Path;

use recite_compiler::{
    BuildCancellation, BuildCheckError, BuildResultFailure, BuildStatusProjection,
    BuildTerminalStatus, FreshnessStatus, PublishFailureReason, PublishNotAttemptedReason,
    PublishOutcome, PublishRefusal, RestartGuidance,
};

use crate::schema_inspection::machine_path;
use crate::structured::data::{ArtifactMetadata, artifact_metadata};

use super::super::build::BuildStatus;
use super::super::recovery::{
    ProjectBuildRecovery, ProjectBuildRecoveryDetail, ProjectBuildRecoveryIoKind,
};
use super::super::wire_types::*;

pub(super) fn build_status(status: BuildTerminalStatus) -> BuildStatusDto {
    match status {
        BuildTerminalStatus::Succeeded => BuildStatusDto::Succeeded,
        BuildTerminalStatus::Failed => BuildStatusDto::Failed,
        BuildTerminalStatus::Stale => BuildStatusDto::Stale,
        BuildTerminalStatus::Cancelled => BuildStatusDto::Cancelled,
        BuildTerminalStatus::Superseded => BuildStatusDto::Superseded,
        _ => BuildStatusDto::Unknown,
    }
}

pub(super) fn status_dto(status: &BuildStatus) -> BuildStatusDto {
    match status {
        BuildStatus::Fresh { .. } => BuildStatusDto::Succeeded,
        BuildStatus::Stale { .. } => BuildStatusDto::Stale,
        BuildStatus::Diagnostics { .. } | BuildStatus::DiagnosticsWithRecovery { .. } => {
            BuildStatusDto::Failed
        }
        BuildStatus::RecoveryRequired { .. } => BuildStatusDto::Succeeded,
        BuildStatus::PublicationFailure { status, .. } => build_status(*status),
    }
}

pub(super) fn outcome_dto(
    status: &BuildStatus,
    projection: &BuildStatusProjection,
) -> BuildOutcomeDto {
    match status {
        BuildStatus::Fresh { .. } => BuildOutcomeDto::Fresh,
        BuildStatus::Stale { .. } => BuildOutcomeDto::Stale,
        BuildStatus::Diagnostics { .. } => BuildOutcomeDto::Diagnostics,
        BuildStatus::DiagnosticsWithRecovery { .. } | BuildStatus::RecoveryRequired { .. } => {
            BuildOutcomeDto::RecoveryRequired
        }
        BuildStatus::PublicationFailure { .. } => build_outcome(projection),
    }
}

pub(super) fn build_outcome(projection: &BuildStatusProjection) -> BuildOutcomeDto {
    if projection
        .failure()
        .is_some_and(|failure| matches!(failure, BuildResultFailure::Freshness { .. }))
    {
        return BuildOutcomeDto::FreshnessFailure;
    }
    match projection.terminal_status() {
        Some(BuildTerminalStatus::Cancelled) => BuildOutcomeDto::Cancelled,
        Some(BuildTerminalStatus::Superseded) => BuildOutcomeDto::Superseded,
        Some(BuildTerminalStatus::Stale) => BuildOutcomeDto::Stale,
        Some(BuildTerminalStatus::Succeeded)
            if projection
                .recovery()
                .is_some_and(|value| !value.targets().is_empty()) =>
        {
            BuildOutcomeDto::RecoveryRequired
        }
        Some(BuildTerminalStatus::Succeeded) => BuildOutcomeDto::Fresh,
        Some(BuildTerminalStatus::Failed)
            if projection.failure().is_some_and(|failure| {
                matches!(failure, BuildResultFailure::Diagnostics { .. })
            }) =>
        {
            BuildOutcomeDto::Diagnostics
        }
        Some(BuildTerminalStatus::Failed)
            if projection.publish().is_some_and(|publish| {
                matches!(
                    publish,
                    PublishOutcome::Partial { .. }
                        | PublishOutcome::Indeterminate { .. }
                        | PublishOutcome::Refused { .. }
                )
            }) || projection.failure().is_some_and(|failure| {
                matches!(
                    failure,
                    BuildResultFailure::Preparation { .. }
                        | BuildResultFailure::InvalidPublication(_)
                )
            }) =>
        {
            BuildOutcomeDto::PublicationFailure
        }
        Some(BuildTerminalStatus::Failed) => BuildOutcomeDto::OperationalFailure,
        None => BuildOutcomeDto::OperationalFailure,
        Some(_) => BuildOutcomeDto::Unknown,
    }
}

pub(super) fn input_keys(projection: &BuildStatusProjection) -> Vec<String> {
    let mut inputs = projection
        .affected_inputs()
        .iter()
        .map(|affected| affected.input().key().as_str().to_owned())
        .collect::<Vec<_>>();
    inputs.sort();
    inputs.dedup();
    inputs
}

pub(super) fn freshness(value: &recite_compiler::FreshnessAssessment) -> FreshnessDto {
    match value.status() {
        FreshnessStatus::Fresh => FreshnessDto::Fresh,
        FreshnessStatus::Stale => FreshnessDto::Stale {
            reasons: value
                .reasons()
                .iter()
                .map(|reason| match reason {
                    recite_compiler::StaleReason::BuildGeneration => {
                        StaleReasonDto::BuildGeneration
                    }
                    recite_compiler::StaleReason::SnapshotGeneration => {
                        StaleReasonDto::SnapshotGeneration
                    }
                    recite_compiler::StaleReason::Fingerprints => StaleReasonDto::Fingerprints,
                    _ => StaleReasonDto::Unknown,
                })
                .collect(),
        },
        FreshnessStatus::Unknown => FreshnessDto::Unknown,
        _ => FreshnessDto::Unknown,
    }
}

pub(super) fn publication(value: &PublishOutcome) -> PublicationDto {
    match value {
        PublishOutcome::NotAttempted { reason } => PublicationDto::NotAttempted {
            reason: not_attempted_reason(*reason),
        },
        PublishOutcome::Published { targets } => PublicationDto::Published {
            targets: target_names(targets),
        },
        PublishOutcome::Partial {
            committed,
            failed,
            remaining,
            recovery,
        } => PublicationDto::Partial {
            committed: target_names(committed),
            failed: failed.as_str().to_owned(),
            remaining: target_names(remaining),
            recovery: target_names(recovery.targets()),
        },
        PublishOutcome::Indeterminate {
            attempted,
            recovery,
        } => PublicationDto::Indeterminate {
            attempted: target_names(attempted),
            recovery: target_names(recovery.targets()),
        },
        PublishOutcome::Refused { reason } => PublicationDto::Refused {
            reason: refusal(*reason),
        },
        _ => PublicationDto::Unknown,
    }
}

fn target_names(targets: &[recite_compiler::BuildTarget]) -> Vec<String> {
    let mut names = targets
        .iter()
        .map(|target| target.as_str().to_owned())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

pub(super) fn artifact_metadata_for_publication(
    project_root: &Path,
    publication: Option<&PublishOutcome>,
    candidates: &[recite_compiler::BuildCandidate],
) -> Result<Vec<ArtifactMetadata>, crate::error::CliError> {
    let targets = match publication {
        Some(PublishOutcome::Published { targets }) => targets,
        Some(PublishOutcome::Partial { committed, .. }) => committed,
        _ => return Ok(Vec::new()),
    };
    let mut targets = target_names(targets);
    targets.sort();
    targets
        .iter()
        .map(|target| {
            let path = project_root.join(target);
            if let Some(candidate) = candidates
                .iter()
                .find(|candidate| candidate.target().as_str() == target)
            {
                return Ok(ArtifactMetadata {
                    path: machine_path(&path),
                    size_bytes: candidate.bytes().len() as u64,
                });
            }
            artifact_metadata(&path)
        })
        .collect()
}

pub(super) fn recovery_record(value: &ProjectBuildRecovery) -> RecoveryDto {
    RecoveryDto {
        marker: machine_path(value.marker()),
        reason: match value.reason() {
            super::super::ProjectBuildRecoveryReason::StageCleanupFailed => {
                RecoveryReasonDto::StageCleanupFailed
            }
            super::super::ProjectBuildRecoveryReason::PublicationIndeterminate => {
                RecoveryReasonDto::PublicationIndeterminate
            }
            super::super::ProjectBuildRecoveryReason::PublicationUncommitted => {
                RecoveryReasonDto::PublicationUncommitted
            }
        },
        detail: match value.detail() {
            ProjectBuildRecoveryDetail::None => None,
            ProjectBuildRecoveryDetail::Io {
                kind, raw_os_error, ..
            } => Some(RecoveryDetailDto::Io {
                kind: recovery_io_kind(kind),
                raw_os_error,
            }),
        },
    }
}

pub(super) fn restart_guidance(_value: Option<RestartGuidance>) -> RestartGuidanceDto {
    host_policy_required()
}

pub(super) const fn host_policy_required() -> RestartGuidanceDto {
    RestartGuidanceDto::HostPolicyRequired {
        decision: "unspecified",
    }
}

pub(super) fn cancellation(value: BuildCancellation) -> CancellationDto {
    match value {
        BuildCancellation::User => CancellationDto::User,
        BuildCancellation::Superseded { by } => CancellationDto::Superseded {
            by_generation: by.as_u64(),
        },
        _ => CancellationDto::Unknown,
    }
}

pub(super) fn failure(value: &BuildResultFailure) -> FailureDto {
    match value {
        BuildResultFailure::Check(reason) => FailureDto::Check {
            reason: match reason {
                BuildCheckError::RequestMismatch => CheckFailureReasonDto::RequestMismatch,
                BuildCheckError::FreshnessMismatch => CheckFailureReasonDto::FreshnessMismatch,
                _ => CheckFailureReasonDto::Unknown,
            },
        },
        BuildResultFailure::Diagnostics { .. } => FailureDto::Diagnostics,
        BuildResultFailure::Engine { reason } => FailureDto::Engine {
            reason: match reason {
                recite_compiler::BuildFailureReason::InvalidOutput => {
                    EngineFailureReasonDto::InvalidOutput
                }
                recite_compiler::BuildFailureReason::Host => EngineFailureReasonDto::Host,
                recite_compiler::BuildFailureReason::Unknown => EngineFailureReasonDto::Unknown,
                _ => EngineFailureReasonDto::Unknown,
            },
        },
        BuildResultFailure::DuplicateTarget { target } => FailureDto::DuplicateTarget {
            target: target.as_str().to_owned(),
        },
        BuildResultFailure::Preparation { target, reason } => FailureDto::Preparation {
            target: target.as_str().to_owned(),
            reason: match reason {
                PublishFailureReason::Rejected => PublishFailureReasonDto::Rejected,
                PublishFailureReason::Storage => PublishFailureReasonDto::Storage,
                PublishFailureReason::Unknown => PublishFailureReasonDto::Unknown,
                _ => PublishFailureReasonDto::Unknown,
            },
        },
        BuildResultFailure::InvalidPublication(_) => FailureDto::InvalidPublication,
        BuildResultFailure::Freshness { .. } => FailureDto::Freshness,
        _ => FailureDto::Unknown,
    }
}

fn not_attempted_reason(value: PublishNotAttemptedReason) -> PublishNotAttemptedReasonDto {
    match value {
        PublishNotAttemptedReason::BuildFailed => PublishNotAttemptedReasonDto::BuildFailed,
        PublishNotAttemptedReason::Cancelled => PublishNotAttemptedReasonDto::Cancelled,
        PublishNotAttemptedReason::Superseded => PublishNotAttemptedReasonDto::Superseded,
        PublishNotAttemptedReason::Stale => PublishNotAttemptedReasonDto::Stale,
        PublishNotAttemptedReason::NoCandidates => PublishNotAttemptedReasonDto::NoCandidates,
        PublishNotAttemptedReason::PreparationFailed => {
            PublishNotAttemptedReasonDto::PreparationFailed
        }
        PublishNotAttemptedReason::InvalidOutcome => PublishNotAttemptedReasonDto::InvalidOutcome,
        _ => PublishNotAttemptedReasonDto::Unknown,
    }
}

fn refusal(value: PublishRefusal) -> PublishRefusalDto {
    match value {
        PublishRefusal::StaleBuildGeneration => PublishRefusalDto::StaleBuildGeneration,
        PublishRefusal::StaleSnapshotGeneration => PublishRefusalDto::StaleSnapshotGeneration,
        PublishRefusal::StaleFingerprints => PublishRefusalDto::StaleFingerprints,
        PublishRefusal::RequestIdentityMismatch => PublishRefusalDto::RequestIdentityMismatch,
        _ => PublishRefusalDto::Unknown,
    }
}

fn recovery_io_kind(value: ProjectBuildRecoveryIoKind) -> RecoveryIoKindDto {
    match value {
        ProjectBuildRecoveryIoKind::AlreadyExists => RecoveryIoKindDto::AlreadyExists,
        ProjectBuildRecoveryIoKind::InvalidInput => RecoveryIoKindDto::InvalidInput,
        ProjectBuildRecoveryIoKind::NotFound => RecoveryIoKindDto::NotFound,
        ProjectBuildRecoveryIoKind::PermissionDenied => RecoveryIoKindDto::PermissionDenied,
        ProjectBuildRecoveryIoKind::Other => RecoveryIoKindDto::Other,
    }
}
