use super::super::SnapshotGeneration;
use super::freshness::{AffectedInput, FreshnessAssessment, RestartGuidance};
use super::identity::BuildGeneration;
use super::publish::{BuildCandidate, PublishOutcome, PublishRefusal};
use super::request::BuildRequest;
use recite_core::Diagnostic;
use std::time::Duration;

/// Non-semantic timing metadata.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct BuildTelemetry {
    duration: Option<Duration>,
}
impl BuildTelemetry {
    #[must_use]
    pub const fn from_duration(duration: Duration) -> Self {
        Self {
            duration: Some(duration),
        }
    }
    #[must_use]
    pub const fn none() -> Self {
        Self { duration: None }
    }
    #[must_use]
    pub const fn duration(&self) -> Option<Duration> {
        self.duration
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BuildTerminalStatus {
    Succeeded,
    Failed,
    Stale,
    Cancelled,
    Superseded,
}
impl std::fmt::Display for BuildTerminalStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Stale => "stale",
            Self::Cancelled => "cancelled",
            Self::Superseded => "superseded",
        })
    }
}

/// Deterministic build output and host handoff data.
#[derive(Clone, Debug, Eq)]
#[non_exhaustive]
pub struct BuildResult {
    status: BuildTerminalStatus,
    generation: BuildGeneration,
    snapshot_generation: SnapshotGeneration,
    affected_inputs: Vec<AffectedInput>,
    diagnostics: Vec<Diagnostic>,
    candidates: Vec<BuildCandidate>,
    freshness: FreshnessAssessment,
    publish: PublishOutcome,
    restart: RestartGuidance,
    telemetry: BuildTelemetry,
}
impl PartialEq for BuildResult {
    fn eq(&self, other: &Self) -> bool {
        self.semantic_eq(other)
    }
}
impl BuildResult {
    pub(crate) fn new(
        status: BuildTerminalStatus,
        request: &BuildRequest,
        diagnostics: Vec<Diagnostic>,
        candidates: Vec<BuildCandidate>,
        freshness: FreshnessAssessment,
        publish: PublishOutcome,
        telemetry: BuildTelemetry,
    ) -> Self {
        Self {
            status,
            generation: request.generation(),
            snapshot_generation: request.snapshot_generation(),
            affected_inputs: request.affected_inputs(),
            diagnostics,
            candidates,
            freshness,
            publish,
            restart: request.restart_guidance(),
            telemetry,
        }
    }
    /// Attach non-semantic host timing metadata after a run completes.
    #[must_use]
    pub fn with_telemetry(mut self, telemetry: BuildTelemetry) -> Self {
        self.telemetry = telemetry;
        self
    }
    #[must_use]
    pub fn semantic_eq(&self, other: &Self) -> bool {
        self.status == other.status
            && self.generation == other.generation
            && self.snapshot_generation == other.snapshot_generation
            && self.affected_inputs == other.affected_inputs
            && self.diagnostics == other.diagnostics
            && self.candidates == other.candidates
            && self.freshness == other.freshness
            && self.publish == other.publish
            && self.restart == other.restart
    }
    #[must_use]
    pub const fn status(&self) -> BuildTerminalStatus {
        self.status
    }
    #[must_use]
    pub const fn generation(&self) -> BuildGeneration {
        self.generation
    }
    #[must_use]
    pub const fn snapshot_generation(&self) -> SnapshotGeneration {
        self.snapshot_generation
    }
    #[must_use]
    pub fn affected_inputs(&self) -> &[AffectedInput] {
        &self.affected_inputs
    }
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    #[must_use]
    pub fn candidates(&self) -> &[BuildCandidate] {
        &self.candidates
    }
    #[must_use]
    pub const fn freshness(&self) -> &FreshnessAssessment {
        &self.freshness
    }
    #[must_use]
    pub const fn publish(&self) -> &PublishOutcome {
        &self.publish
    }
    #[must_use]
    pub const fn restart_guidance(&self) -> RestartGuidance {
        self.restart
    }
    #[must_use]
    pub const fn telemetry(&self) -> &BuildTelemetry {
        &self.telemetry
    }
}

/// Current generation/snapshot/fingerprint authority at publication time.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BuildAuthority {
    latest_generation: BuildGeneration,
    snapshot_generation: SnapshotGeneration,
    fingerprints: super::identity::BuildFingerprintSet,
}
impl BuildAuthority {
    #[must_use]
    pub fn from_request(request: &BuildRequest) -> Self {
        Self {
            latest_generation: request.generation(),
            snapshot_generation: request.snapshot_generation(),
            fingerprints: request.fingerprints().clone(),
        }
    }
    #[must_use]
    pub fn new(
        latest_generation: BuildGeneration,
        snapshot_generation: SnapshotGeneration,
        fingerprints: super::identity::BuildFingerprintSet,
    ) -> Self {
        Self {
            latest_generation,
            snapshot_generation,
            fingerprints,
        }
    }
    #[must_use]
    pub const fn latest_generation(&self) -> BuildGeneration {
        self.latest_generation
    }
    #[must_use]
    pub const fn snapshot_generation(&self) -> SnapshotGeneration {
        self.snapshot_generation
    }
    #[must_use]
    pub const fn fingerprints(&self) -> &super::identity::BuildFingerprintSet {
        &self.fingerprints
    }
    pub(crate) fn refusal_for(&self, request: &BuildRequest) -> Option<PublishRefusal> {
        if request.generation() != self.latest_generation {
            Some(PublishRefusal::StaleBuildGeneration)
        } else if request.snapshot_generation() != self.snapshot_generation {
            Some(PublishRefusal::StaleSnapshotGeneration)
        } else if request.fingerprints() != &self.fingerprints {
            Some(PublishRefusal::StaleFingerprints)
        } else {
            None
        }
    }
}
