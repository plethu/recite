use recite_core::{ChoiceId, EffectId, LocaleId};

use crate::DialogueError;
use crate::event::DialogueEffectRequest;
use crate::session_snapshot::{
    DialogueDeferredEffectSnapshot, DialogueSessionSnapshotConversionError,
};
use crate::traversal::{AssetView, dialogue_effect_request};

pub(super) fn restore_choice_ids(
    asset: AssetView<'_>,
    field: &'static str,
    values: &[String],
) -> Result<Vec<ChoiceId>, DialogueError> {
    values
        .iter()
        .map(|value| {
            let choice_id = choice_id(value)?;
            asset.choice_by_id(&choice_id)?;
            Ok(choice_id)
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| match error {
            DialogueError::MalformedCompiledAsset { reason } => invalid_snapshot(format!(
                "{field} reference invalid compiled choice: {reason}"
            )),
            other => other,
        })
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

pub(super) fn restore_effects(
    asset: AssetView<'_>,
    snapshots: &[DialogueDeferredEffectSnapshot],
) -> Result<Vec<DialogueEffectRequest>, DialogueError> {
    snapshots
        .iter()
        .map(|snapshot| restore_effect(asset, snapshot))
        .collect()
}

fn restore_effect(
    asset: AssetView<'_>,
    snapshot: &DialogueDeferredEffectSnapshot,
) -> Result<DialogueEffectRequest, DialogueError> {
    let effect_id = effect_id(&snapshot.id)?;
    let effect = asset
        .deferred_effect_by_id(&effect_id)
        .map_err(|error| match error {
            DialogueError::MalformedCompiledAsset { reason } => {
                invalid_snapshot(format!("deferred effect reference is invalid: {reason}"))
            }
            other => other,
        })?;

    dialogue_effect_request(asset, effect)
}

pub(super) fn restore_locale(value: Option<&str>) -> Result<Option<LocaleId>, DialogueError> {
    value.map(LocaleId::new).transpose().map_err(core_error)
}

pub(super) fn choice_id(value: &str) -> Result<ChoiceId, DialogueError> {
    ChoiceId::new(value).map_err(core_error)
}

pub(super) fn effect_id(value: &str) -> Result<EffectId, DialogueError> {
    EffectId::new(value).map_err(core_error)
}

fn core_error(error: impl std::fmt::Display) -> DialogueError {
    invalid_snapshot(error.to_string())
}

pub(super) fn invalid_snapshot(reason: impl Into<String>) -> DialogueError {
    DialogueError::InvalidSessionSnapshot {
        reason: reason.into(),
        source: None,
    }
}

pub(super) fn invalid_snapshot_with_source(
    error: DialogueSessionSnapshotConversionError,
) -> DialogueError {
    let reason = error.to_string();
    DialogueError::InvalidSessionSnapshot {
        reason,
        source: Some(Box::new(error)),
    }
}
