#[path = "support/preview.rs"]
mod preview_support;

use preview_support::asset;
use recite_runtime::{PreviewEvent, PreviewInputs, PreviewOptions, PreviewSession, PreviewStatus};

fn changed(mut asset: recite_core::CompiledDialogue) -> recite_core::CompiledDialogue {
    asset.lines[0].source_text.push_str(" changed");
    asset
}

#[test]
fn restart_requirement_is_orthogonal_to_ready_choice_condition_and_end() {
    let ready_asset = asset(":: start default\n> line@12345678901234567890\n  Line.\n-> END\n");
    let mut ready = PreviewSession::new(&ready_asset, None, PreviewOptions::new()).expect("ready");
    let output = ready
        .assess_asset(&changed(ready_asset.clone()))
        .expect("assess");
    assert!(matches!(
        output.events(),
        [PreviewEvent::RestartRequired { .. }]
    ));
    assert!(matches!(ready.state().status(), PreviewStatus::Ready));
    assert!(ready.state().restart_required().is_some());

    let choice_asset = asset(
        ":: start default\n> prompt@12345678901234567890\n  Prompt.\n  ? go@12345678901234567891\n    Go.\n    -> END\n",
    );
    let mut choice =
        PreviewSession::new(&choice_asset, None, PreviewOptions::new()).expect("choice");
    choice.step(PreviewInputs::new());
    choice
        .assess_asset(&changed(choice_asset.clone()))
        .expect("assess");
    assert!(matches!(
        choice.state().status(),
        PreviewStatus::WaitingForChoice { .. }
    ));
    assert!(choice.state().restart_required().is_some());

    let condition_asset = asset(
        ":: start default\n:if trusts(player)\n  > yes@12345678901234567890\n    Yes.\n-> END\n",
    );
    let mut condition =
        PreviewSession::new(&condition_asset, None, PreviewOptions::new()).expect("condition");
    condition.step(PreviewInputs::new());
    condition
        .assess_asset(&changed(condition_asset.clone()))
        .expect("assess");
    assert!(matches!(
        condition.state().status(),
        PreviewStatus::WaitingForCondition { .. }
    ));
    assert!(condition.state().restart_required().is_some());

    let end_asset = asset(":: start default\n> line@12345678901234567890\n  Line.\n-> END\n");
    let mut ended = PreviewSession::new(&end_asset, None, PreviewOptions::new()).expect("ended");
    ended.step(PreviewInputs::new());
    ended.step(PreviewInputs::new());
    ended
        .assess_asset(&changed(end_asset.clone()))
        .expect("assess");
    assert!(matches!(ended.state().status(), PreviewStatus::Ended));
    assert!(ended.state().restart_required().is_some());
}

#[test]
fn restarting_old_asset_does_not_clear_replacement_requirement() {
    let active = asset(":: start default\n> line@12345678901234567890\n  Line.\n-> END\n");
    let mut preview = PreviewSession::new(&active, None, PreviewOptions::new()).expect("start");
    let candidate = changed(active.clone());
    preview.assess_asset(&candidate).expect("assess");
    let replacement = preview.state().restart_required().cloned();
    preview.dispatch(
        recite_runtime::PreviewCommand::Restart,
        PreviewInputs::new(),
    );
    assert_eq!(preview.state().restart_required(), replacement.as_ref());
    assert!(matches!(preview.state().status(), PreviewStatus::Ready));
}
