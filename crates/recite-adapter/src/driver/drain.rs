use recite_core::compiled::CompiledDialogue;
use recite_runtime::{
    DialogueContext, DialogueEffectMode, DialogueEffectRequest, DialogueError, DialogueEvent,
    DialogueSession, LocaleResolution, next_with,
};

use crate::{AdapterError, AdapterErrorKind, AdapterResult};

// The runtime bounds each next_with call. A line/effect loop can nevertheless
// make an adapter operation emit forever across individually bounded calls.
const MAX_BATCH_EVENTS: usize = 10_000;

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
        if events.len() == MAX_BATCH_EVENTS {
            return Err(AdapterError::with_detail(
                AdapterErrorKind::DialogueFault,
                format!("adapter operation exceeded {MAX_BATCH_EVENTS} output events"),
            ));
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
