pub(super) const fn key(id: super::MsgId) -> Option<&'static str> {
    match id {
        super::MsgId::WatchBuilding => Some("watch-building"),
        super::MsgId::WatchWaitingForChanges => Some("watch-waiting-for-changes"),
        super::MsgId::WatchRebuilding => Some("watch-rebuilding"),
        super::MsgId::WatchBuildSucceeded => Some("watch-build-succeeded"),
        super::MsgId::WatchBuildFailedWaiting => Some("watch-build-failed-waiting"),
        super::MsgId::WatchBuildFailed => Some("watch-build-failed"),
        super::MsgId::WatchBuildFailedPartial => Some("watch-build-failed-partial"),
        super::MsgId::WatchBuildFailedIndeterminate => Some("watch-build-failed-indeterminate"),
        super::MsgId::WatchBuildFailedRefused => Some("watch-build-failed-refused"),
        super::MsgId::WatchBuildFailedNotAttempted => Some("watch-build-failed-not-attempted"),
        super::MsgId::WatchBuildFailedPublished => Some("watch-build-failed-published"),
        super::MsgId::WatchBuildFailedUnsupported => Some("watch-build-failed-unsupported"),
        super::MsgId::WatchBuildFailedPartialWithFailure => {
            Some("watch-build-failed-partial-with-failure")
        }
        super::MsgId::WatchBuildFailedIndeterminateWithFailure => {
            Some("watch-build-failed-indeterminate-with-failure")
        }
        super::MsgId::WatchBuildFailedRefusedWithFailure => {
            Some("watch-build-failed-refused-with-failure")
        }
        super::MsgId::WatchBuildFailedNotAttemptedWithFailure => {
            Some("watch-build-failed-not-attempted-with-failure")
        }
        super::MsgId::WatchBuildFailedPublishedWithFailure => {
            Some("watch-build-failed-published-with-failure")
        }
        super::MsgId::WatchBuildFailedUnsupportedWithFailure => {
            Some("watch-build-failed-unsupported-with-failure")
        }
        super::MsgId::WatchBuildStatusSucceeded => Some("watch-build-status-succeeded"),
        super::MsgId::WatchBuildStatusFailed => Some("watch-build-status-failed"),
        super::MsgId::WatchBuildStatusStale => Some("watch-build-status-stale"),
        super::MsgId::WatchBuildStatusCancelled => Some("watch-build-status-cancelled"),
        super::MsgId::WatchBuildStatusSuperseded => Some("watch-build-status-superseded"),
        super::MsgId::WatchBuildStatusUnknown => Some("watch-build-status-unknown"),
        super::MsgId::WatchBuildRecoveryTargetsEmpty => Some("watch-build-recovery-targets-empty"),
        super::MsgId::WatchBuildRecoveryTargetsList => Some("watch-build-recovery-targets-list"),
        super::MsgId::WatchBuildFailureCheckRequestMismatch => {
            Some("watch-build-failure-check-request-mismatch")
        }
        super::MsgId::WatchBuildFailureCheckFreshnessMismatch => {
            Some("watch-build-failure-check-freshness-mismatch")
        }
        super::MsgId::WatchBuildFailureCheckUnknown => Some("watch-build-failure-check-unknown"),
        super::MsgId::WatchBuildFailureDiagnostics => Some("watch-build-failure-diagnostics"),
        super::MsgId::WatchBuildFailureUnknown => Some("watch-build-failure-unknown"),
        super::MsgId::WatchBuildFailureEngineInvalidOutput => {
            Some("watch-build-failure-engine-invalid-output")
        }
        super::MsgId::WatchBuildFailureEngineHost => Some("watch-build-failure-engine-host"),
        super::MsgId::WatchBuildFailureEngineUnknown => Some("watch-build-failure-engine-unknown"),
        super::MsgId::WatchBuildFailureDuplicateTarget => {
            Some("watch-build-failure-duplicate-target")
        }
        super::MsgId::WatchBuildFailurePreparation => Some("watch-build-failure-preparation"),
        super::MsgId::WatchBuildFailureReasonRejected => {
            Some("watch-build-failure-reason-rejected")
        }
        super::MsgId::WatchBuildFailureReasonStorage => Some("watch-build-failure-reason-storage"),
        super::MsgId::WatchBuildFailureReasonUnknown => Some("watch-build-failure-reason-unknown"),
        super::MsgId::WatchBuildFailureInvalidPublishedPartition => {
            Some("watch-build-failure-invalid-published-partition")
        }
        super::MsgId::WatchBuildFailureInvalidPartialPartition => {
            Some("watch-build-failure-invalid-partial-partition")
        }
        super::MsgId::WatchBuildFailureInvalidRecoveryTarget => {
            Some("watch-build-failure-invalid-recovery-target")
        }
        super::MsgId::WatchBuildFailureInvalidNotCommitted => {
            Some("watch-build-failure-invalid-not-committed")
        }
        super::MsgId::WatchBuildFailureInvalidUnknown => {
            Some("watch-build-failure-invalid-unknown")
        }
        super::MsgId::WatchBuildFailureRefusalStaleBuildGeneration => {
            Some("watch-build-failure-refusal-stale-build-generation")
        }
        super::MsgId::WatchBuildFailureRefusalStaleSnapshotGeneration => {
            Some("watch-build-failure-refusal-stale-snapshot-generation")
        }
        super::MsgId::WatchBuildFailureRefusalStaleFingerprints => {
            Some("watch-build-failure-refusal-stale-fingerprints")
        }
        super::MsgId::WatchBuildFailureRefusalRequestIdentityMismatch => {
            Some("watch-build-failure-refusal-request-identity-mismatch")
        }
        super::MsgId::WatchBuildFailureRefusalUnknown => {
            Some("watch-build-failure-refusal-unknown")
        }
        super::MsgId::WatchBuildFailureNotAttemptedBuildFailed => {
            Some("watch-build-failure-not-attempted-build-failed")
        }
        super::MsgId::WatchBuildFailureNotAttemptedCancelled => {
            Some("watch-build-failure-not-attempted-cancelled")
        }
        super::MsgId::WatchBuildFailureNotAttemptedSuperseded => {
            Some("watch-build-failure-not-attempted-superseded")
        }
        super::MsgId::WatchBuildFailureNotAttemptedStale => {
            Some("watch-build-failure-not-attempted-stale")
        }
        super::MsgId::WatchBuildFailureNotAttemptedNoCandidates => {
            Some("watch-build-failure-not-attempted-no-candidates")
        }
        super::MsgId::WatchBuildFailureNotAttemptedPreparationFailed => {
            Some("watch-build-failure-not-attempted-preparation-failed")
        }
        super::MsgId::WatchBuildFailureNotAttemptedInvalidOutcome => {
            Some("watch-build-failure-not-attempted-invalid-outcome")
        }
        super::MsgId::WatchBuildFailureNotAttemptedUnknown => {
            Some("watch-build-failure-not-attempted-unknown")
        }
        _ => None,
    }
}
