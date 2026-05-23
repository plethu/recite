use recite_core::{CompiledEffectMode, EffectId, EffectIndex};

use crate::{DialogueError, DialogueEvent, DialogueSession, EffectAck};

use super::AssetView;
use super::flow::next_statement_after;
use super::output::dialogue_effect_request;

pub fn acknowledge_effect(
    session: &mut DialogueSession,
    effect_id: recite_core::EffectId,
    _ack: EffectAck,
) -> Result<(), DialogueError> {
    let Some(pending) = &session.pending_effect else {
        return Err(DialogueError::NoEffectPending { effect: effect_id });
    };
    if pending.id != effect_id {
        return Err(DialogueError::WrongEffectAcknowledgement {
            expected: pending.id.clone(),
            actual: effect_id,
        });
    }

    session.pending_effect = None;
    Ok(())
}

pub(super) fn handle_effect(
    asset: AssetView<'_>,
    session: &mut DialogueSession,
    effect_index: EffectIndex,
) -> Result<Option<DialogueEvent>, DialogueError> {
    let effect = asset.effect_at(effect_index)?;
    let request = dialogue_effect_request(asset, effect)?;

    let next_statement = next_statement_after(session.next_statement)?;
    session.next_statement = next_statement;

    match effect.mode {
        CompiledEffectMode::Deferred => {
            session.deferred_effects.push(request);
            Ok(None)
        }
        CompiledEffectMode::Immediate => Ok(Some(DialogueEvent::Effect(runtime_effect_request(
            session, request,
        )?))),
        CompiledEffectMode::Blocking => {
            let request = runtime_effect_request(session, request)?;
            session.pending_effect = Some(request.clone());
            Ok(Some(DialogueEvent::Effect(request)))
        }
    }
}

fn runtime_effect_request(
    session: &DialogueSession,
    mut request: crate::DialogueEffectRequest,
) -> Result<crate::DialogueEffectRequest, DialogueError> {
    request.id = EffectId::new(format!(
        "{}#{}",
        request.id.as_str(),
        session.next_trace_counter()?
    ))
    .map_err(|error| DialogueError::MalformedCompiledAsset {
        reason: format!("runtime effect request id is invalid: {error}"),
    })?;
    Ok(request)
}
