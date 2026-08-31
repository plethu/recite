use recite_compiler::{
    BuildCheckError, BuildFailureReason, BuildResultFailure, PublishFailureReason,
    PublishNotAttemptedReason, PublishOutcomeError, PublishRefusal,
};

pub(super) fn format_failure_reason(
    messages: &crate::i18n::Messages,
    failure: &BuildResultFailure,
) -> String {
    match failure {
        BuildResultFailure::Check(error) => match error {
            BuildCheckError::RequestMismatch => {
                messages.text(crate::i18n::MsgId::WatchBuildFailureCheckRequestMismatch)
            }
            BuildCheckError::FreshnessMismatch => {
                messages.text(crate::i18n::MsgId::WatchBuildFailureCheckFreshnessMismatch)
            }
            _ => messages.text(crate::i18n::MsgId::WatchBuildFailureCheckUnknown),
        },
        BuildResultFailure::Diagnostics { .. } => {
            messages.text(crate::i18n::MsgId::WatchBuildFailureDiagnostics)
        }
        BuildResultFailure::Engine { reason } => {
            let id = match reason {
                BuildFailureReason::InvalidOutput => {
                    crate::i18n::MsgId::WatchBuildFailureEngineInvalidOutput
                }
                BuildFailureReason::Host => crate::i18n::MsgId::WatchBuildFailureEngineHost,
                BuildFailureReason::Unknown => crate::i18n::MsgId::WatchBuildFailureEngineUnknown,
                _ => crate::i18n::MsgId::WatchBuildFailureEngineUnknown,
            };
            messages.text(id)
        }
        BuildResultFailure::DuplicateTarget { target } => messages.format(
            crate::i18n::MsgId::WatchBuildFailureDuplicateTarget,
            [("target", target.to_string())],
        ),
        BuildResultFailure::Preparation { target, reason } => messages.format(
            crate::i18n::MsgId::WatchBuildFailurePreparation,
            [
                ("target", target.to_string()),
                ("reason", format_publish_failure_reason(messages, *reason)),
            ],
        ),
        BuildResultFailure::InvalidPublication(error) => {
            let id = match error {
                PublishOutcomeError::PublishedPartition => {
                    crate::i18n::MsgId::WatchBuildFailureInvalidPublishedPartition
                }
                PublishOutcomeError::PartialPartition => {
                    crate::i18n::MsgId::WatchBuildFailureInvalidPartialPartition
                }
                PublishOutcomeError::RecoveryTarget => {
                    crate::i18n::MsgId::WatchBuildFailureInvalidRecoveryTarget
                }
                PublishOutcomeError::NotCommitted => {
                    crate::i18n::MsgId::WatchBuildFailureInvalidNotCommitted
                }
                _ => crate::i18n::MsgId::WatchBuildFailureInvalidUnknown,
            };
            messages.text(id)
        }
        _ => messages.text(crate::i18n::MsgId::WatchBuildFailureUnknown),
    }
}

fn format_publish_failure_reason(
    messages: &crate::i18n::Messages,
    reason: PublishFailureReason,
) -> String {
    let id = match reason {
        PublishFailureReason::Rejected => crate::i18n::MsgId::WatchBuildFailureReasonRejected,
        PublishFailureReason::Storage => crate::i18n::MsgId::WatchBuildFailureReasonStorage,
        PublishFailureReason::Unknown => crate::i18n::MsgId::WatchBuildFailureReasonUnknown,
        _ => crate::i18n::MsgId::WatchBuildFailureReasonUnknown,
    };
    messages.text(id)
}

pub(super) fn format_refusal(messages: &crate::i18n::Messages, reason: PublishRefusal) -> String {
    let id = match reason {
        PublishRefusal::StaleBuildGeneration => {
            crate::i18n::MsgId::WatchBuildFailureRefusalStaleBuildGeneration
        }
        PublishRefusal::StaleSnapshotGeneration => {
            crate::i18n::MsgId::WatchBuildFailureRefusalStaleSnapshotGeneration
        }
        PublishRefusal::StaleFingerprints => {
            crate::i18n::MsgId::WatchBuildFailureRefusalStaleFingerprints
        }
        PublishRefusal::RequestIdentityMismatch => {
            crate::i18n::MsgId::WatchBuildFailureRefusalRequestIdentityMismatch
        }
        _ => crate::i18n::MsgId::WatchBuildFailureRefusalUnknown,
    };
    messages.text(id)
}

pub(super) fn format_not_attempted(
    messages: &crate::i18n::Messages,
    reason: PublishNotAttemptedReason,
) -> String {
    let id = match reason {
        PublishNotAttemptedReason::BuildFailed => {
            crate::i18n::MsgId::WatchBuildFailureNotAttemptedBuildFailed
        }
        PublishNotAttemptedReason::Cancelled => {
            crate::i18n::MsgId::WatchBuildFailureNotAttemptedCancelled
        }
        PublishNotAttemptedReason::Superseded => {
            crate::i18n::MsgId::WatchBuildFailureNotAttemptedSuperseded
        }
        PublishNotAttemptedReason::Stale => crate::i18n::MsgId::WatchBuildFailureNotAttemptedStale,
        PublishNotAttemptedReason::NoCandidates => {
            crate::i18n::MsgId::WatchBuildFailureNotAttemptedNoCandidates
        }
        PublishNotAttemptedReason::PreparationFailed => {
            crate::i18n::MsgId::WatchBuildFailureNotAttemptedPreparationFailed
        }
        PublishNotAttemptedReason::InvalidOutcome => {
            crate::i18n::MsgId::WatchBuildFailureNotAttemptedInvalidOutcome
        }
        _ => crate::i18n::MsgId::WatchBuildFailureNotAttemptedUnknown,
    };
    messages.text(id)
}
