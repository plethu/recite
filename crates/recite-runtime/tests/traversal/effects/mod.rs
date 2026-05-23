use super::*;

mod blocking;
mod deferred;
mod immediate;

fn assert_effect(
    event: Result<DialogueEvent, DialogueError>,
    function: &str,
    mode: DialogueEffectMode,
) -> DialogueEffectRequest {
    let DialogueEvent::Effect(effect) = event.expect("effect event succeeds") else {
        panic!("expected effect event");
    };

    assert_eq!(effect.function, function);
    assert_eq!(effect.mode, mode);
    effect
}
