#[path = "support/preview.rs"]
mod preview_support;

use preview_support::asset;
use recite_runtime::{
    DialogueEffectMode, EffectAck, PreviewError, PreviewEvent, PreviewInputs, PreviewOptions,
    PreviewSession, PreviewStatus, PreviewTranscriptEvent,
};

#[test]
fn prompt_choice_effect_ack_and_snapshot_restore_keep_stable_identity() {
    let asset = asset(concat!(
        ":: start default\n",
        "> prompt@12345678901234567890\n  Choose.\n",
        "  ? work@12345678901234567891\n    Work.\n    -> work\n",
        ":: work\n",
        "! deferred record(work)\n",
        "! immediate sound(work)\n",
        "! blocking overlay(work)\n",
        "-> END\n",
    ));
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("start");
    let prompt = preview.step(PreviewInputs::new());
    let snapshot = preview.snapshot().expect("prompt snapshot");
    assert!(matches!(prompt.events(), [PreviewEvent::Prompt(_)]));
    let choice = match &prompt.events()[0] {
        PreviewEvent::Prompt(prompt) => prompt.identity().choices()[0].clone(),
        _ => unreachable!("asserted prompt"),
    };
    let selected = preview.choose(choice.clone(), PreviewInputs::new());
    assert!(selected.events().iter().any(|event| matches!(
        event,
        PreviewEvent::DeferredEffectScheduled(effect)
            if effect.mode == DialogueEffectMode::Deferred
    )));
    assert!(selected.events().iter().any(|event| matches!(
        event,
        PreviewEvent::EffectRequested(effect)
            if effect.mode == DialogueEffectMode::Immediate
    )));
    let blocking = preview.step(PreviewInputs::new());
    let effect = match blocking.events().last() {
        Some(PreviewEvent::EffectRequested(effect)) => effect.clone(),
        event => panic!("expected blocking effect, got {event:?}"),
    };
    assert!(matches!(
        preview.state().status(),
        PreviewStatus::WaitingForEffect { .. }
    ));
    let blocked_snapshot = preview.snapshot().expect("effect snapshot");
    let before = preview.session().clone();
    let wrong = preview.acknowledge(
        recite_core::EffectId::new("wrong").expect("id"),
        EffectAck::Completed,
    );
    assert_eq!(*preview.session(), before);
    assert!(matches!(
        wrong.events(),
        [PreviewEvent::Error(PreviewError::Runtime(_))]
    ));
    preview.acknowledge(
        effect.id.clone(),
        EffectAck::Failed {
            reason: "closed".to_owned(),
        },
    );
    let after = preview.step(PreviewInputs::new());
    assert!(matches!(after.events(), [PreviewEvent::End { .. }]));
    let mut restored = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("start");
    restored.restore(snapshot).expect("restore prompt");
    let selected_again = restored.choose(choice, PreviewInputs::new());
    assert!(
        selected_again
            .events()
            .iter()
            .any(|event| matches!(event, PreviewEvent::EffectRequested(_)))
    );
    let mut effect_restored =
        PreviewSession::new(&asset, None, PreviewOptions::new()).expect("start");
    effect_restored
        .restore(blocked_snapshot)
        .expect("restore effect");
    let emitted = effect_restored.step(PreviewInputs::new());
    assert!(matches!(
        emitted.events(),
        [PreviewEvent::EffectRequested(request)] if request.id == effect.id
    ));
    assert!(
        effect_restored
            .transcript()
            .events()
            .iter()
            .any(|event| matches!(event, PreviewTranscriptEvent::EffectRequested(_)))
    );
}
