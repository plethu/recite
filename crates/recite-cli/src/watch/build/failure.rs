use recite_compiler::{BuildResultFailure, BuildTerminalStatus, PublishOutcome};

use super::failure_reasons::{format_failure_reason, format_not_attempted, format_refusal};

pub(crate) fn format_failure(
    messages: &crate::i18n::Messages,
    status: BuildTerminalStatus,
    failure: Option<&BuildResultFailure>,
    outcome: &PublishOutcome,
) -> String {
    let status = format_status(messages, status);
    let failure = failure.map(|failure| format_failure_reason(messages, failure));
    match outcome {
        PublishOutcome::Partial {
            failed, recovery, ..
        } => match failure {
            Some(failure) => messages.format(
                crate::i18n::MsgId::WatchBuildFailedPartialWithFailure,
                [
                    ("status", status.clone()),
                    ("failed", failed.to_string()),
                    ("recovery", format_targets(messages, recovery.targets())),
                    ("failure", failure),
                ],
            ),
            None => messages.format(
                crate::i18n::MsgId::WatchBuildFailedPartial,
                [
                    ("status", status.clone()),
                    ("failed", failed.to_string()),
                    ("recovery", format_targets(messages, recovery.targets())),
                ],
            ),
        },
        PublishOutcome::Indeterminate { recovery, .. } => match failure {
            Some(failure) => messages.format(
                crate::i18n::MsgId::WatchBuildFailedIndeterminateWithFailure,
                [
                    ("status", status.clone()),
                    ("recovery", format_targets(messages, recovery.targets())),
                    ("failure", failure),
                ],
            ),
            None => messages.format(
                crate::i18n::MsgId::WatchBuildFailedIndeterminate,
                [
                    ("status", status.clone()),
                    ("recovery", format_targets(messages, recovery.targets())),
                ],
            ),
        },
        PublishOutcome::Refused { reason } => match failure {
            Some(failure) => messages.format(
                crate::i18n::MsgId::WatchBuildFailedRefusedWithFailure,
                [
                    ("status", status.clone()),
                    ("reason", format_refusal(messages, *reason)),
                    ("failure", failure),
                ],
            ),
            None => messages.format(
                crate::i18n::MsgId::WatchBuildFailedRefused,
                [
                    ("status", status.clone()),
                    ("reason", format_refusal(messages, *reason)),
                ],
            ),
        },
        PublishOutcome::NotAttempted { reason } => match failure {
            Some(failure) => messages.format(
                crate::i18n::MsgId::WatchBuildFailedNotAttemptedWithFailure,
                [
                    ("status", status.clone()),
                    ("reason", format_not_attempted(messages, *reason)),
                    ("failure", failure),
                ],
            ),
            None => messages.format(
                crate::i18n::MsgId::WatchBuildFailedNotAttempted,
                [
                    ("status", status.clone()),
                    ("reason", format_not_attempted(messages, *reason)),
                ],
            ),
        },
        PublishOutcome::Published { .. } => match failure {
            Some(failure) => messages.format(
                crate::i18n::MsgId::WatchBuildFailedPublishedWithFailure,
                [("status", status.clone()), ("failure", failure)],
            ),
            None => messages.format(
                crate::i18n::MsgId::WatchBuildFailedPublished,
                [("status", status.clone())],
            ),
        },
        _ => match failure {
            Some(failure) => messages.format(
                crate::i18n::MsgId::WatchBuildFailedUnsupportedWithFailure,
                [("status", status.clone()), ("failure", failure)],
            ),
            None => messages.format(
                crate::i18n::MsgId::WatchBuildFailedUnsupported,
                [("status", status.clone())],
            ),
        },
    }
}

fn format_status(messages: &crate::i18n::Messages, status: BuildTerminalStatus) -> String {
    let id = match status {
        BuildTerminalStatus::Succeeded => crate::i18n::MsgId::WatchBuildStatusSucceeded,
        BuildTerminalStatus::Failed => crate::i18n::MsgId::WatchBuildStatusFailed,
        BuildTerminalStatus::Stale => crate::i18n::MsgId::WatchBuildStatusStale,
        BuildTerminalStatus::Cancelled => crate::i18n::MsgId::WatchBuildStatusCancelled,
        BuildTerminalStatus::Superseded => crate::i18n::MsgId::WatchBuildStatusSuperseded,
        _ => crate::i18n::MsgId::WatchBuildStatusUnknown,
    };
    messages.text(id)
}

fn format_targets(
    messages: &crate::i18n::Messages,
    targets: &[recite_compiler::BuildTarget],
) -> String {
    if targets.is_empty() {
        return messages.text(crate::i18n::MsgId::WatchBuildRecoveryTargetsEmpty);
    }
    targets
        .iter()
        .map(|target| {
            messages.format(
                crate::i18n::MsgId::WatchBuildRecoveryTargetsList,
                [("target", escape_target(target.as_str()))],
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn escape_target(target: &str) -> String {
    let mut escaped = String::with_capacity(target.len());
    for character in target.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() || matches!(character, '\u{2028}' | '\u{2029}') => {
                escaped.push_str(&format!("\\u{{{:04x}}}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped
}
