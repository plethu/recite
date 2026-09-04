use recite_compiler::{BuildResultFailure, BuildTelemetry, BuildTerminalStatus, PublishOutcome};

use super::super::ProjectBuildRecovery;

/// The presentation boundary's compact view of one coordinated build.
#[derive(Debug, Eq)]
pub(crate) enum BuildStatus {
    Fresh {
        asset_count: usize,
        telemetry: BuildTelemetry,
    },
    Stale {
        asset_count: usize,
        recovery: Vec<ProjectBuildRecovery>,
        telemetry: BuildTelemetry,
    },
    Diagnostics {
        telemetry: BuildTelemetry,
    },
    DiagnosticsWithRecovery {
        recovery: Vec<ProjectBuildRecovery>,
        telemetry: BuildTelemetry,
    },
    RecoveryRequired {
        asset_count: usize,
        recovery: Vec<ProjectBuildRecovery>,
        telemetry: BuildTelemetry,
    },
    PublicationFailure {
        status: BuildTerminalStatus,
        failure: Option<BuildResultFailure>,
        outcome: PublishOutcome,
        recovery: Vec<ProjectBuildRecovery>,
        telemetry: BuildTelemetry,
    },
}

impl PartialEq for BuildStatus {
    fn eq(&self, other: &Self) -> bool {
        self.semantic_eq(other)
    }
}

impl BuildStatus {
    /// Compare presentation-relevant build state without host timing, which
    /// is deliberately non-semantic and varies between invocations.
    fn semantic_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Fresh {
                    asset_count: left, ..
                },
                Self::Fresh {
                    asset_count: right, ..
                },
            ) => left == right,
            (
                Self::Stale {
                    asset_count: left_count,
                    recovery: left_recovery,
                    ..
                },
                Self::Stale {
                    asset_count: right_count,
                    recovery: right_recovery,
                    ..
                },
            ) => left_count == right_count && left_recovery == right_recovery,
            (Self::Diagnostics { .. }, Self::Diagnostics { .. }) => true,
            (
                Self::DiagnosticsWithRecovery { recovery: left, .. },
                Self::DiagnosticsWithRecovery {
                    recovery: right, ..
                },
            ) => left == right,
            (
                Self::RecoveryRequired {
                    asset_count: left_count,
                    recovery: left_recovery,
                    ..
                },
                Self::RecoveryRequired {
                    asset_count: right_count,
                    recovery: right_recovery,
                    ..
                },
            ) => left_count == right_count && left_recovery == right_recovery,
            (
                Self::PublicationFailure {
                    status: left_status,
                    failure: left_failure,
                    outcome: left_outcome,
                    recovery: left_recovery,
                    ..
                },
                Self::PublicationFailure {
                    status: right_status,
                    failure: right_failure,
                    outcome: right_outcome,
                    recovery: right_recovery,
                    ..
                },
            ) => {
                left_status == right_status
                    && left_failure == right_failure
                    && left_outcome == right_outcome
                    && left_recovery == right_recovery
            }
            _ => false,
        }
    }

    pub(crate) const fn telemetry(&self) -> &BuildTelemetry {
        match self {
            Self::Fresh { telemetry, .. }
            | Self::Stale { telemetry, .. }
            | Self::Diagnostics { telemetry }
            | Self::DiagnosticsWithRecovery { telemetry, .. }
            | Self::RecoveryRequired { telemetry, .. }
            | Self::PublicationFailure { telemetry, .. } => telemetry,
        }
    }

    pub(crate) fn recovery(&self) -> &[ProjectBuildRecovery] {
        match self {
            Self::Stale { recovery, .. }
            | Self::DiagnosticsWithRecovery { recovery, .. }
            | Self::RecoveryRequired { recovery, .. }
            | Self::PublicationFailure { recovery, .. } => recovery,
            Self::Fresh { .. } | Self::Diagnostics { .. } => &[],
        }
    }
}
