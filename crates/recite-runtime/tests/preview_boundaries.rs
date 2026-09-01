#[path = "support/preview.rs"]
mod preview_support;

use preview_support::asset;
use recite_runtime::{PreviewEvent, PreviewInputs, PreviewOptions, PreviewSession, PreviewStatus};

fn changed(mut asset: recite_core::CompiledDialogue) -> recite_core::CompiledDialogue {
    asset.lines[0].source_text.push_str(" changed");
    asset.lines[0].authored_source_text.push_str(" changed");
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

#[test]
fn malformed_candidate_revision_fails_without_mutating_restart_state() {
    let active = asset(":: start default\n> line@12345678901234567890\n  Line.\n-> END\n");
    let mut malformed = active.clone();
    malformed.metadata.push(recite_core::CompiledMetadataEntry {
        key: "score".to_owned(),
        value: recite_core::Value::Scalar(recite_core::ScalarValue::Float(f64::NAN)),
        source_map: None,
    });
    let mut preview = PreviewSession::new(&active, None, PreviewOptions::new()).expect("start");
    let before = preview.state().clone();

    let error = preview
        .assess_asset(&malformed)
        .expect_err("malformed candidate must be refused");

    assert!(matches!(
        error,
        recite_runtime::PreviewError::AssetRevisionFailed { .. }
    ));
    assert_eq!(preview.state(), &before);
    assert!(preview.state().restart_required().is_none());
}

#[test]
fn malformed_candidate_index_fails_without_mutating_restart_state() {
    let active = asset(":: start default\n> line@12345678901234567890\n  Line.\n-> END\n");
    let mut malformed = active.clone();
    malformed.default_block = recite_core::BlockIndex::new(99);
    let mut preview = PreviewSession::new(&active, None, PreviewOptions::new()).expect("start");
    let before = preview.state().clone();

    let error = preview
        .assess_asset(&malformed)
        .expect_err("malformed candidate must be refused");

    assert!(matches!(
        error,
        recite_runtime::PreviewError::AssetRevisionFailed { .. }
    ));
    assert_eq!(preview.state(), &before);
    assert!(preview.state().restart_required().is_none());
}

#[test]
fn mutable_candidate_interpolation_and_reason_invariants_are_transactional() {
    let active = asset(
        ":: start default\n> prompt@12345678901234567890\n  Prompt.\n  ? go@12345678901234567891\n    Go.\n    -> END\n",
    );

    let mut mismatched_choice = active.clone();
    mismatched_choice.choices[0]
        .source_text
        .push_str(" {missing}");
    assert_invalid_candidate(&active, &mismatched_choice);

    let mut legacy_line = active.clone();
    legacy_line.lines[0].source_text = "{missing}".to_owned();
    legacy_line.lines[0].authored_source_text = "{missing}".to_owned();
    legacy_line.lines[0].interpolation_mode = recite_core::CompiledInterpolationMode::Legacy;
    assert_invalid_candidate(&active, &legacy_line);

    let mut malformed_reason = active.clone();
    malformed_reason
        .availability_reasons
        .push(recite_core::CompiledAvailabilityReason {
            id: recite_core::AvailabilityReasonId::new("weight").expect("valid reason id"),
            template: "Weight {value}.".to_owned(),
        });
    malformed_reason.condition_availability_reasons.push(
        recite_core::CompiledConditionAvailabilityReason {
            function: "can_answer".to_owned(),
            reason: recite_core::AvailabilityReasonId::new("weight").expect("valid reason id"),
            args: vec![recite_core::CompiledAvailabilityReasonArgBinding {
                name: "value".to_owned(),
                value: recite_core::CompiledAvailabilityReasonArgValue::Literal(
                    recite_core::ScalarValue::Float(f64::NAN),
                ),
            }],
        },
    );
    assert_invalid_candidate(&active, &malformed_reason);
}

fn assert_invalid_candidate(
    active: &recite_core::CompiledDialogue,
    candidate: &recite_core::CompiledDialogue,
) {
    let Some(mut preview) = PreviewSession::new(active, None, PreviewOptions::new()).ok() else {
        panic!("valid test fixture must create a preview session");
    };
    let before = preview.state().clone();
    let result = preview.assess_asset(candidate);
    assert!(matches!(
        result,
        Err(recite_runtime::PreviewError::AssetRevisionFailed { .. })
    ));
    assert_eq!(preview.state(), &before);
}
