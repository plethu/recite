use recite_core::{CompiledEffectMode, EffectIndex};

use crate::{DialogueError, DialogueSession};

use super::AssetView;
use super::flow::next_statement_after;
use super::output::{dialogue_effect_request, effect_mode};

pub(super) fn collect_deferred_effect(
    asset: AssetView<'_>,
    session: &mut DialogueSession,
    effect_index: EffectIndex,
) -> Result<(), DialogueError> {
    let effect = asset.effect_at(effect_index)?;
    let mode = effect_mode(effect.mode);
    if !matches!(effect.mode, CompiledEffectMode::Deferred) {
        return Err(DialogueError::UnsupportedEffectMode { mode });
    }

    let next_statement = next_statement_after(session.next_statement)?;
    session
        .deferred_effects
        .push(dialogue_effect_request(asset, effect)?);
    session.next_statement = next_statement;

    Ok(())
}
