use super::*;

pub(super) fn assert_snapshot_encoding(session: &recite_runtime::DialogueSession) {
    assert_eq!(
        encode_session_messagepack(session).expect("borrowed session encodes"),
        rmp_serde::to_vec(&snapshot_session(session)).expect("owned snapshot encodes"),
        "borrowed encoding preserves every owned snapshot field and its order",
    );
}

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
