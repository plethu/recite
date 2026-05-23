use super::*;

pub(super) fn assert_effect(
    event: Result<DialogueEvent, DialogueError>,
    function: &str,
) -> DialogueEffectRequest {
    let DialogueEvent::Effect(effect) = event.expect("effect event succeeds") else {
        panic!("expected effect event");
    };

    assert_eq!(effect.function, function);
    effect
}
