use recite_core::{CompiledEffectMode, EffectId, EffectIndex};

use crate::session::PendingEffect;
use crate::{DialogueError, DialogueEvent, DialogueSession, EffectAck};

use super::AssetView;
use super::flow::next_statement_after;
use super::output::dialogue_effect_request;

/// Acknowledge the blocking effect currently pending on a session.
///
/// The acknowledgement ID must match the exact runtime effect request emitted by
/// traversal. The runtime clears the pending effect but does not execute or
/// interpret the game-side operation; callers decide whether a completed or
/// failed acknowledgement affects their own game state.
pub fn acknowledge_effect(
    session: &mut DialogueSession,
    effect_id: recite_core::EffectId,
    _ack: EffectAck,
) -> Result<(), DialogueError> {
    let Some(pending) = &session.pending_effect else {
        return Err(DialogueError::NoEffectPending { effect: effect_id });
    };
    if pending.request.id != effect_id {
        return Err(DialogueError::WrongEffectAcknowledgement {
            expected: pending.request.id.clone(),
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

    let effect_statement = session.next_statement;
    let next_statement = next_statement_after(session.next_statement)?;
    session.next_statement = next_statement;

    match effect.mode {
        CompiledEffectMode::Deferred => {
            session.deferred_effects.push(request);
            Ok(None)
        }
        CompiledEffectMode::Immediate => emit_effect_for_next_trace_event(session, request),
        CompiledEffectMode::Blocking => {
            let request = request_for_next_trace_event(session, request)?;
            session.pending_effect = Some(PendingEffect {
                statement: effect_statement,
                request: request.clone(),
                reemit_on_next: false,
            });
            Ok(Some(DialogueEvent::Effect(request)))
        }
    }
}

fn emit_effect_for_next_trace_event(
    session: &DialogueSession,
    request: crate::DialogueEffectRequest,
) -> Result<Option<DialogueEvent>, DialogueError> {
    request_for_next_trace_event(session, request)
        .map(|request| Some(DialogueEvent::Effect(request)))
}

fn request_for_next_trace_event(
    session: &DialogueSession,
    request: crate::DialogueEffectRequest,
) -> Result<crate::DialogueEffectRequest, DialogueError> {
    runtime_effect_request_for_trace_counter(request, session.next_trace_counter()?)
}

pub(crate) fn runtime_effect_request_for_trace_counter(
    mut request: crate::DialogueEffectRequest,
    trace_counter: u64,
) -> Result<crate::DialogueEffectRequest, DialogueError> {
    request.id =
        EffectId::new(format!("{}#{}", request.id.as_str(), trace_counter)).map_err(|error| {
            DialogueError::MalformedCompiledAsset {
                reason: format!("runtime effect request id is invalid: {error}"),
            }
        })?;
    Ok(request)
}
