use recite_core::{ChoiceId, LocaleId};

use crate::DialogueError;
use crate::event::{DialogueEffectMode, DialogueEffectRequest};
use crate::session_snapshot::{
    DialogueEffectRequestSnapshot, DialogueSessionSnapshotConversionError,
    effect_request_from_snapshot,
};

/// Choice IDs in saved history are authored identifiers, kept even when a
/// patch has since removed the choice.
pub(super) fn restore_choice_ids(values: &[String]) -> Result<Vec<ChoiceId>, DialogueError> {
    values.iter().map(|value| choice_id(value)).collect()
}

pub(super) fn snapshot_reference<T>(
    field: &'static str,
    result: Result<T, DialogueError>,
) -> Result<T, DialogueError> {
    result.map_err(|error| match error {
        DialogueError::MalformedCompiledAsset { reason } => {
            invalid_snapshot(format!("{field} references invalid asset data: {reason}"))
        }
        other => other,
    })
}

/// Saved deferred effects are self-contained requests. They are delivered as
/// saved, so a session resumed in a patched asset, and saved again, still
/// restores: compiled effect IDs are positional and need not exist any more.
pub(super) fn restore_effects(
    snapshots: &[DialogueEffectRequestSnapshot],
) -> Result<Vec<DialogueEffectRequest>, DialogueError> {
    snapshots
        .iter()
        .map(|snapshot| {
            let request =
                effect_request_from_snapshot(snapshot).map_err(invalid_snapshot_with_source)?;
            if request.mode != DialogueEffectMode::Deferred {
                return Err(invalid_snapshot(format!(
                    "deferred effect request `{}` is not a deferred effect",
                    request.id
                )));
            }
            Ok(request)
        })
        .collect()
}

pub(crate) fn restore_locale(value: Option<&str>) -> Result<Option<LocaleId>, DialogueError> {
    value.map(LocaleId::new).transpose().map_err(core_error)
}

pub(crate) fn choice_id(value: &str) -> Result<ChoiceId, DialogueError> {
    ChoiceId::new(value).map_err(core_error)
}

fn core_error(error: impl std::fmt::Display) -> DialogueError {
    invalid_snapshot(error.to_string())
}

pub(crate) fn invalid_snapshot(reason: impl Into<String>) -> DialogueError {
    DialogueError::InvalidSessionSnapshot {
        reason: reason.into(),
        source: None,
    }
}

pub(crate) fn invalid_snapshot_with_source(
    error: DialogueSessionSnapshotConversionError,
) -> DialogueError {
    let reason = error.to_string();
    DialogueError::InvalidSessionSnapshot {
        reason,
        source: Some(Box::new(error)),
    }
}
