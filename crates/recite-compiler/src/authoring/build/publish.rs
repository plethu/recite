pub use super::candidates::{BuildCandidate, BuildTarget, BuildTargetError};
use super::fingerprints::BuildFingerprintSet;
use super::identity::BuildGeneration;
use super::request::BuildRequest;
use super::request_identity::BuildRequestIdentity;

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
    RequestIdentityMismatch,
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
    /// Commit ran but its returned partition was not trustworthy. All listed
    /// targets are conservatively considered attempted and require recovery.
    Indeterminate {
        attempted: Vec<BuildTarget>,
        recovery: RecoveryNeeded,
    },
    Refused {
        reason: PublishRefusal,
    },
}

/// Reducer identity for a publisher-owned, non-cloneable prepared handle.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PreparedPublishIdentity {
    request_identity: BuildRequestIdentity,
    candidates: Vec<BuildCandidate>,
}
impl PreparedPublishIdentity {
    /// Construct identity data; this is not a publication handle.
    pub fn for_request(request: &BuildRequest, candidates: Vec<BuildCandidate>) -> Self {
        Self {
            request_identity: BuildRequestIdentity::from_request(request),
            candidates,
        }
    }
    #[must_use]
    pub const fn generation(&self) -> BuildGeneration {
        self.request_identity.generation()
    }
    #[must_use]
    pub const fn snapshot_generation(&self) -> SnapshotGeneration {
        self.request_identity.snapshot_generation()
    }
    #[must_use]
    pub const fn fingerprints(&self) -> &BuildFingerprintSet {
        self.request_identity.fingerprints()
    }
    #[must_use]
    pub fn candidates(&self) -> &[BuildCandidate] {
        &self.candidates
    }
    #[must_use]
    pub const fn request_identity(&self) -> &BuildRequestIdentity {
        &self.request_identity
    }
}
/// Publisher-owned preparation handle consumed by commit or abort.
pub trait BuildPreparedHandle {
    fn identity(&self) -> PreparedPublishIdentity;
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
            Self::Indeterminate { .. } => Err(PublishOutcomeError::NotCommitted),
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
