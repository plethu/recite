use super::super::SnapshotGeneration;
use super::fingerprints::BuildFingerprintSet;
use super::freshness::RestartGuidance;
use super::identity::{BuildGeneration, BuildInputPolicy};
use super::request::BuildRequest;

/// Complete request authority used by checks, prepared batches, results, and fences.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BuildRequestIdentity {
    generation: BuildGeneration,
    snapshot_generation: SnapshotGeneration,
    policy: BuildInputPolicy,
    fingerprints: BuildFingerprintSet,
    restart: RestartGuidance,
}
impl BuildRequestIdentity {
    #[must_use]
    pub fn from_request(request: &BuildRequest) -> Self {
        Self {
            generation: request.generation(),
            snapshot_generation: request.snapshot_generation(),
            policy: request.input_policy(),
            fingerprints: request.fingerprints().clone(),
            restart: request.restart_guidance(),
        }
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
    pub const fn policy(&self) -> BuildInputPolicy {
        self.policy
    }
    #[must_use]
    pub const fn fingerprints(&self) -> &BuildFingerprintSet {
        &self.fingerprints
    }
    #[must_use]
    pub const fn restart_guidance(&self) -> RestartGuidance {
        self.restart
    }
    #[must_use]
    pub(crate) fn matches_request(&self, request: &BuildRequest) -> bool {
        self == &Self::from_request(request)
    }
}
