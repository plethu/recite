use recite_compiler::{BuildResultFailure, BuildTerminalStatus, PublishOutcome};
use recite_ui::{UiArg, UiArgs};

use super::super::ProjectBuildRecovery;
use super::failure_reasons::{
    format_failure_reason, format_not_attempted, format_recovery_reason, format_refusal,
};

pub(crate) fn format_failure_with_recovery(
    messages: &crate::i18n::Messages,
    status: BuildTerminalStatus,
    failure: Option<&BuildResultFailure>,
    outcome: &PublishOutcome,
    recovery: &[ProjectBuildRecovery],
) -> String {
    let status = format_status(messages, status);
    let failure = failure.map(|failure| format_failure_reason(messages, failure));
    let records = format_recovery_summary(messages, recovery);
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
                    ("records", records.clone()),
                ],
            ),
            None => messages.format(
                crate::i18n::MsgId::WatchBuildFailedPartial,
                [
                    ("status", status.clone()),
                    ("failed", failed.to_string()),
                    ("recovery", format_targets(messages, recovery.targets())),
                    ("records", records.clone()),
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
                    ("records", records.clone()),
                ],
            ),
            None => messages.format(
                crate::i18n::MsgId::WatchBuildFailedIndeterminate,
                [
                    ("status", status.clone()),
                    ("recovery", format_targets(messages, recovery.targets())),
                    ("records", records.clone()),
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
                    ("records", records.clone()),
                ],
            ),
            None => messages.format(
                crate::i18n::MsgId::WatchBuildFailedRefused,
                [
                    ("status", status.clone()),
                    ("reason", format_refusal(messages, *reason)),
                    ("records", records.clone()),
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
                    ("records", records.clone()),
                ],
            ),
            None => messages.format(
                crate::i18n::MsgId::WatchBuildFailedNotAttempted,
                [
                    ("status", status.clone()),
                    ("reason", format_not_attempted(messages, *reason)),
                    ("records", records.clone()),
                ],
            ),
        },
        PublishOutcome::Published { .. } => match failure {
            Some(failure) => messages.format(
                crate::i18n::MsgId::WatchBuildFailedPublishedWithFailure,
                [
                    ("status", status.clone()),
                    ("failure", failure),
                    ("records", records.clone()),
                ],
            ),
            None => messages.format(
                crate::i18n::MsgId::WatchBuildFailedPublished,
                [("status", status.clone()), ("records", records.clone())],
            ),
        },
        _ => match failure {
            Some(failure) => messages.format(
                crate::i18n::MsgId::WatchBuildFailedUnsupportedWithFailure,
                [
                    ("status", status.clone()),
                    ("failure", failure),
                    ("records", records.clone()),
                ],
            ),
            None => messages.format(
                crate::i18n::MsgId::WatchBuildFailedUnsupported,
                [("status", status.clone()), ("records", records)],
            ),
        },
    }
}

pub(crate) fn format_recovery_required(
    messages: &crate::i18n::Messages,
    asset_count: usize,
    recovery: &[ProjectBuildRecovery],
) -> String {
    let mut args = UiArgs::new();
    args.insert("count".to_owned(), UiArg::from(asset_count));
    args.insert(
        "records".to_owned(),
        UiArg::from(format_recovery_summary(messages, recovery)),
    );
    messages.format_args(crate::i18n::MsgId::WatchBuildRecoveryRequired, &args)
}

pub(crate) fn format_recovery_notice(
    messages: &crate::i18n::Messages,
    recovery: &[ProjectBuildRecovery],
) -> String {
    messages.format(
        crate::i18n::MsgId::WatchBuildRecoveryNotice,
        [("records", format_recovery_summary(messages, recovery))],
    )
}

fn format_recovery_summary(
    messages: &crate::i18n::Messages,
    recovery: &[ProjectBuildRecovery],
) -> String {
    let mut records = recovery.to_vec();
    records.sort();
    records.dedup();
    let items = records
        .iter()
        .map(|record| {
            messages.format(
                crate::i18n::MsgId::WatchBuildRecoveryRecord,
                [
                    ("marker", escape_target(&record.marker().to_string_lossy())),
                    ("reason", format_recovery_reason(messages, record.reason())),
                ],
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let mut args = UiArgs::new();
    args.insert("count".to_owned(), UiArg::from(records.len()));
    args.insert("items".to_owned(), UiArg::from(items));
    messages.format_args(crate::i18n::MsgId::WatchBuildRecoverySummary, &args)
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
