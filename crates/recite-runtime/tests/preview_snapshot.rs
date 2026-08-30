#[path = "support/preview.rs"]
mod preview_support;

use preview_support::asset;
use recite_core::LocaleId;
use recite_runtime::{
    ConditionAnswer, ConditionValue, PreviewEvent, PreviewInputs, PreviewOptions, PreviewSession,
};

#[test]
fn encoded_snapshot_restores_options_block_and_condition_counter() {
    let asset = asset(concat!(
        ":: start default\n",
        "> first@12345678901234567890\n  First.\n",
        ":if trusts(player)\n",
        "  > second@12345678901234567891\n    Second.\n",
        "-> END\n",
        ":: alternate\n",
        "> alternate_line@12345678901234567892\n  Alternate.\n-> END\n",
    ));
    let options = PreviewOptions::new()
        .with_locale(LocaleId::new("fr-FR").expect("locale"))
        .with_variant("formal");
    let source = PreviewSession::new(&asset, Some("alternate"), options.clone()).expect("source");
    let snapshot = source.snapshot().expect("snapshot");
    let decoded = recite_runtime::PreviewSnapshot::decode(&snapshot.encode().expect("encode"))
        .expect("decode");
    let mut restored = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("receiver");
    restored.restore(decoded).expect("restore");
    assert_eq!(
        restored.state().locale().map(LocaleId::as_str),
        Some("fr-FR")
    );
    assert_eq!(restored.trace().variant(), Some("formal"));
    let restarted = restored.dispatch(
        recite_runtime::PreviewCommand::Restart,
        PreviewInputs::new(),
    );
    assert!(matches!(
        restarted.events(),
        [PreviewEvent::Restarted { block: Some(block), .. }] if block.as_str() == "alternate"
    ));

    let mut original = PreviewSession::new(&asset, None, PreviewOptions::new()).expect("original");
    original.step(PreviewInputs::new());
    let saved = original.snapshot().expect("saved");
    let mut continued =
        PreviewSession::new(&asset, None, PreviewOptions::new()).expect("continued");
    continued.restore(saved).expect("restore saved");
    let left = original.step(PreviewInputs::new());
    let right = continued.step(PreviewInputs::new());
    let left_request = match &left.events()[0] {
        PreviewEvent::ConditionRequested(request) => request,
        event => panic!("expected condition request, got {event:?}"),
    };
    let right_request = match &right.events()[0] {
        PreviewEvent::ConditionRequested(request) => request,
        event => panic!("expected condition request, got {event:?}"),
    };
    assert_eq!(left_request.id(), right_request.id());
    let answer = ConditionAnswer::Value(ConditionValue::Bool(true));
    assert_eq!(
        original
            .answer(left_request.id(), answer.clone(), PreviewInputs::new())
            .events(),
        continued
            .answer(right_request.id(), answer, PreviewInputs::new())
            .events()
    );
}
