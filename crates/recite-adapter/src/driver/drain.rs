use recite_core::compiled::CompiledDialogue;
use recite_runtime::{
    DialogueContext, DialogueEffectMode, DialogueEffectRequest, DialogueError, DialogueEvent,
    DialogueSession, LocaleResolution, next_with,
};

use crate::AdapterResult;

pub(super) fn drain_from_next(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    context: &dyn DialogueContext,
    resolution: LocaleResolution<'_>,
) -> AdapterResult<Vec<DialogueEvent>> {
    match next_with(asset, session, context, resolution) {
        Ok(event) => drain_after_event(asset, session, context, resolution, event),
        Err(DialogueError::PromptPending { .. } | DialogueError::EffectPending { .. }) => {
            Ok(Vec::new())
        }
        Err(error) => Err(error.into()),
    }
}

pub(super) fn drain_after_event(
    asset: &CompiledDialogue,
    session: &mut DialogueSession,
    context: &dyn DialogueContext,
    resolution: LocaleResolution<'_>,
    first: DialogueEvent,
) -> AdapterResult<Vec<DialogueEvent>> {
    let mut events = Vec::new();
    let mut event = first;
    loop {
        let keep_going = matches!(
            &event,
            DialogueEvent::Line(_)
                | DialogueEvent::Effect(DialogueEffectRequest {
                    mode: DialogueEffectMode::Immediate,
                    ..
                })
        );
        events.push(event);
        if !keep_going {
            return Ok(events);
        }
        match next_with(asset, session, context, resolution) {
            Ok(next) => event = next,
            Err(DialogueError::PromptPending { .. } | DialogueError::EffectPending { .. }) => {
                return Ok(events);
            }
            Err(error) => return Err(error.into()),
        }
    }
}
