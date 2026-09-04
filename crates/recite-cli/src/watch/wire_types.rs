use crate::schema_inspection::MachinePathProjection;
use crate::structured::data::ArtifactMetadata;
use crate::structured::errors::StructuredError;
use recite_core::DiagnosticRecord;
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct BuildStartedData {
    pub(super) generation: u64,
    pub(super) trigger: BuildTriggerDto,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum BuildTriggerDto {
    Initial,
    InputChanged,
}

#[derive(Serialize)]
pub(super) struct BuildCompletedData {
    pub(super) generation: u64,
    pub(super) snapshot_generation: Option<u64>,
    pub(super) status: BuildStatusDto,
    pub(super) outcome: BuildOutcomeDto,
    pub(super) inputs: Vec<String>,
    pub(super) diagnostics: Vec<DiagnosticRecord>,
    pub(super) artifacts: Vec<ArtifactMetadata>,
    pub(super) freshness: FreshnessDto,
    pub(super) publication: PublicationDto,
    pub(super) recovery: Vec<RecoveryDto>,
    pub(super) restart_guidance: RestartGuidanceDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) cancellation: Option<CancellationDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) failure: Option<FailureDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<StructuredError>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum BuildStatusDto {
    Succeeded,
    Failed,
    Stale,
    Cancelled,
    Superseded,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum BuildOutcomeDto {
    Fresh,
    Diagnostics,
    Stale,
    RecoveryRequired,
    FreshnessFailure,
    OperationalFailure,
    PublicationFailure,
    Unknown,
    Cancelled,
    Superseded,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum FreshnessDto {
    Fresh,
    Stale { reasons: Vec<StaleReasonDto> },
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum StaleReasonDto {
    BuildGeneration,
    SnapshotGeneration,
    Fingerprints,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum PublicationDto {
    NotAttempted {
        reason: PublishNotAttemptedReasonDto,
    },
    Published {
        targets: Vec<String>,
    },
    Partial {
        committed: Vec<String>,
        failed: String,
        remaining: Vec<String>,
        recovery: Vec<String>,
    },
    Indeterminate {
        attempted: Vec<String>,
        recovery: Vec<String>,
    },
    Refused {
        reason: PublishRefusalDto,
    },
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PublishNotAttemptedReasonDto {
    BuildFailed,
    Cancelled,
    Superseded,
    Stale,
    NoCandidates,
    PreparationFailed,
    InvalidOutcome,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PublishRefusalDto {
    StaleBuildGeneration,
    StaleSnapshotGeneration,
    StaleFingerprints,
    RequestIdentityMismatch,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct RecoveryDto {
    pub(super) marker: MachinePathProjection,
    pub(super) reason: RecoveryReasonDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) detail: Option<RecoveryDetailDto>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RecoveryReasonDto {
    StageCleanupFailed,
    PublicationIndeterminate,
    PublicationUncommitted,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum RecoveryDetailDto {
    Io {
        kind: RecoveryIoKindDto,
        raw_os_error: Option<i32>,
    },
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RecoveryIoKindDto {
    AlreadyExists,
    InvalidInput,
    NotFound,
    PermissionDenied,
    Other,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum RestartGuidanceDto {
    HostPolicyRequired { decision: &'static str },
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum CancellationDto {
    User,
    Superseded { by_generation: u64 },
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum FailureDto {
    Check {
        reason: CheckFailureReasonDto,
    },
    Diagnostics,
    Engine {
        reason: EngineFailureReasonDto,
    },
    DuplicateTarget {
        target: String,
    },
    Preparation {
        target: String,
        reason: PublishFailureReasonDto,
    },
    InvalidPublication,
    Freshness,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CheckFailureReasonDto {
    RequestMismatch,
    FreshnessMismatch,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum EngineFailureReasonDto {
    InvalidOutput,
    Host,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PublishFailureReasonDto {
    Rejected,
    Storage,
    Unknown,
}
