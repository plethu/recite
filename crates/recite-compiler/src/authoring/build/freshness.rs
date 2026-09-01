use super::fingerprints::{BuildFingerprintSet, BuildInputFingerprint};
use super::publish::RecoveryNeeded;

/// Why a canonical input is affected by a build.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum AffectedInputReason {
    Added,
    Changed,
    Dependency,
    Unknown,
}

/// One deterministic affected-input record.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct AffectedInput {
    input: BuildInputFingerprint,
    reason: AffectedInputReason,
}

impl AffectedInput {
    pub(crate) fn new(input: BuildInputFingerprint) -> Self {
        Self {
            input,
            reason: AffectedInputReason::Changed,
        }
    }
    #[must_use]
    pub const fn input(&self) -> &BuildInputFingerprint {
        &self.input
    }
    #[must_use]
    pub const fn reason(&self) -> AffectedInputReason {
        self.reason
    }
}

/// Host policy guidance after a successful build.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum RestartGuidance {
    NotApplicable,
    ReloadForNextSessionOnly,
    RestartRequired,
    RejectUntilSessionEnds,
    Unknown,
}

/// A completed source/schema/compiler freshness assessment.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum FreshnessStatus {
    Fresh,
    Stale,
    Unknown,
}

/// Why a candidate or completed assessment is stale.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum StaleReason {
    BuildGeneration,
    SnapshotGeneration,
    Fingerprints,
}

/// Typed reason that a post-publication freshness recheck could not finish.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum FreshnessFailureReason {
    RecheckFailed,
}
impl std::fmt::Display for FreshnessFailureReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::RecheckFailed => "freshness recheck failed",
        })
    }
}

/// Host-provided final freshness outcome for a successfully published build.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FreshnessFinalization {
    Fresh {
        assessment: FreshnessAssessment,
        diagnostics: Vec<recite_core::Diagnostic>,
        recovery: Option<RecoveryNeeded>,
    },
    Stale {
        assessment: FreshnessAssessment,
        diagnostics: Vec<recite_core::Diagnostic>,
        recovery: Option<RecoveryNeeded>,
    },
    Indeterminate {
        assessment: FreshnessAssessment,
        diagnostics: Vec<recite_core::Diagnostic>,
        recovery: Option<RecoveryNeeded>,
        reason: FreshnessFailureReason,
    },
}

/// Freshness remains a field separate from terminal stale state.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct FreshnessAssessment {
    status: FreshnessStatus,
    expected: BuildFingerprintSet,
    reasons: Vec<StaleReason>,
}

impl FreshnessAssessment {
    #[must_use]
    pub fn not_assessed(expected: BuildFingerprintSet) -> Self {
        Self {
            status: FreshnessStatus::Unknown,
            expected,
            reasons: Vec::new(),
        }
    }
    #[must_use]
    pub fn fresh(expected: BuildFingerprintSet) -> Self {
        Self {
            status: FreshnessStatus::Fresh,
            expected,
            reasons: Vec::new(),
        }
    }
    #[must_use]
    pub fn stale(expected: BuildFingerprintSet, mut reasons: Vec<StaleReason>) -> Self {
        reasons.sort_unstable();
        reasons.dedup();
        Self {
            status: FreshnessStatus::Stale,
            expected,
            reasons,
        }
    }
    #[must_use]
    pub const fn status(&self) -> FreshnessStatus {
        self.status
    }
    #[must_use]
    pub const fn expected(&self) -> &BuildFingerprintSet {
        &self.expected
    }
    #[must_use]
    pub fn reasons(&self) -> &[StaleReason] {
        &self.reasons
    }
}
