pub use super::candidates::{BuildCandidate, BuildTarget, BuildTargetError};

/// Targets requiring host-owned repair after a partial multi-target commit.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct RecoveryNeeded {
    targets: Vec<BuildTarget>,
}
impl RecoveryNeeded {
    #[must_use]
    pub fn for_targets(mut targets: Vec<BuildTarget>) -> Self {
        targets.sort();
        targets.dedup();
        Self { targets }
    }
    #[must_use]
    pub fn targets(&self) -> &[BuildTarget] {
        &self.targets
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PublishNotAttemptedReason {
    BuildFailed,
    Cancelled,
    Superseded,
    Stale,
    NoCandidates,
    PreparationFailed,
    InvalidOutcome,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PublishRefusal {
    StaleBuildGeneration,
    StaleSnapshotGeneration,
    StaleFingerprints,
}

/// Publication outcome. `Partial` is explicit because global atomicity is not promised.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum PublishOutcome {
    NotAttempted {
        reason: PublishNotAttemptedReason,
    },
    Published {
        targets: Vec<BuildTarget>,
    },
    Partial {
        committed: Vec<BuildTarget>,
        failed: BuildTarget,
        remaining: Vec<BuildTarget>,
        recovery: RecoveryNeeded,
    },
    Refused {
        reason: PublishRefusal,
    },
}

/// A fully prepared batch bound to one request identity. It is consumed by commit.
#[derive(Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PreparedPublish {
    generation: BuildGeneration,
    snapshot_generation: SnapshotGeneration,
    fingerprints: BuildFingerprintSet,
    candidates: Vec<BuildCandidate>,
}
/// Cloneable reducer identity for a non-cloneable prepared handle.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PreparedPublishIdentity {
    generation: BuildGeneration,
    snapshot_generation: SnapshotGeneration,
    fingerprints: BuildFingerprintSet,
    candidates: Vec<BuildCandidate>,
}
impl PreparedPublish {
    pub fn new(request: &BuildRequest, candidates: Vec<BuildCandidate>) -> Self {
        Self {
            generation: request.generation(),
            snapshot_generation: request.snapshot_generation(),
            fingerprints: request.fingerprints().clone(),
            candidates,
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
    pub const fn fingerprints(&self) -> &BuildFingerprintSet {
        &self.fingerprints
    }
    #[must_use]
    pub fn candidates(&self) -> &[BuildCandidate] {
        &self.candidates
    }
    #[must_use]
    pub fn identity(&self) -> PreparedPublishIdentity {
        PreparedPublishIdentity {
            generation: self.generation,
            snapshot_generation: self.snapshot_generation,
            fingerprints: self.fingerprints.clone(),
            candidates: self.candidates.clone(),
        }
    }
}
impl PreparedPublishIdentity {
    #[must_use]
    pub const fn generation(&self) -> BuildGeneration {
        self.generation
    }
    #[must_use]
    pub const fn snapshot_generation(&self) -> SnapshotGeneration {
        self.snapshot_generation
    }
    #[must_use]
    pub const fn fingerprints(&self) -> &BuildFingerprintSet {
        &self.fingerprints
    }
    #[must_use]
    pub fn candidates(&self) -> &[BuildCandidate] {
        &self.candidates
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PublishAbortReason {
    Cancelled,
    Superseded,
    PreparationFailed,
    Stale,
    Invalid,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum PublishFailure {
    #[error("could not prepare build target {target}: {reason}")]
    Preparation {
        target: BuildTarget,
        reason: PublishFailureReason,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum PublishOutcomeError {
    #[error("published target partition does not match prepared batch")]
    PublishedPartition,
    #[error("partial publication target partition is not disjoint and exhaustive")]
    PartialPartition,
    #[error("publication recovery target is not justified by the partial outcome")]
    RecoveryTarget,
    #[error("publication returned an outcome without committing the prepared batch")]
    NotCommitted,
}

impl PublishOutcome {
    pub(crate) fn validate_against(
        &self,
        prepared: &PreparedPublishIdentity,
    ) -> Result<(), PublishOutcomeError> {
        let expected = prepared
            .candidates()
            .iter()
            .map(|candidate| candidate.target().clone())
            .collect::<Vec<_>>();
        match self {
            Self::Published { targets } if same_targets(targets, &expected) => Ok(()),
            Self::Published { .. } => Err(PublishOutcomeError::PublishedPartition),
            Self::Partial {
                committed,
                failed,
                remaining,
                recovery,
            } => {
                let mut all = committed.clone();
                all.push(failed.clone());
                all.extend(remaining.iter().cloned());
                if !same_targets(&all, &expected) || has_duplicates(&all) {
                    return Err(PublishOutcomeError::PartialPartition);
                }
                if recovery.targets().iter().any(|target| {
                    !committed
                        .iter()
                        .chain(std::iter::once(failed))
                        .any(|item| item == target)
                }) {
                    return Err(PublishOutcomeError::RecoveryTarget);
                }
                Ok(())
            }
            Self::NotAttempted { .. } | Self::Refused { .. } => {
                Err(PublishOutcomeError::NotCommitted)
            }
        }
    }
}

fn same_targets(left: &[BuildTarget], right: &[BuildTarget]) -> bool {
    let mut left = left.to_vec();
    let mut right = right.to_vec();
    left.sort();
    right.sort();
    left == right
}

fn has_duplicates(targets: &[BuildTarget]) -> bool {
    let mut sorted = targets.to_vec();
    sorted.sort();
    sorted.windows(2).any(|pair| pair[0] == pair[1])
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum PublishFailureReason {
    Rejected,
    Storage,
    Unknown,
}
impl std::fmt::Display for PublishFailureReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Rejected => "rejected",
            Self::Storage => "storage failure",
            Self::Unknown => "unknown failure",
        })
    }
}
use super::super::SnapshotGeneration;
use super::fingerprints::BuildFingerprintSet;
use super::identity::BuildGeneration;
use super::request::BuildRequest;
