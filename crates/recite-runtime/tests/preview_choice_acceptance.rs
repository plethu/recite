#[path = "support/preview.rs"]
mod preview_support;

use preview_support::asset;
use recite_runtime::{
    ConditionAnswer, ConditionValue, PreviewError, PreviewEvent, PreviewInputs, PreviewOptions,
    PreviewSession, PreviewStatus, PreviewTranscriptEvent,
};

fn branch_asset() -> recite_core::CompiledDialogue {
    asset(concat!(
        ":: start default\n",
        "> intro@18c570b9af4d973ba876\n",
        "  Choose.\n",
        "  ? go@c491f4cbe1944ebc5bc5\n",
        "    Go.\n",
        "    -> branch\n",
        ":: branch\n",
        ":if trusts(player)\n",
        "  > done@d491f4cbe1944ebc5bc5\n",
        "    Done.\n",
        "-> END\n",
    ))
}

fn choice_prompt(preview: &mut PreviewSession<'_>) -> recite_runtime::PreviewPrompt {
    match preview.step(PreviewInputs::default()).events() {
        [PreviewEvent::Prompt(prompt)] => prompt.clone(),
        events => panic!("expected choice prompt, got {events:?}"),
    }
}

fn pending_branch_condition(
    preview: &mut PreviewSession<'_>,
) -> (
    recite_core::ChoiceId,
    recite_runtime::PreviewConditionRequest,
) {
    let prompt = choice_prompt(preview);
    let choice_id = prompt.choices()[0].id.clone();
    let output = preview.choose(choice_id.clone(), PreviewInputs::default());
    let request = match output.events() {
        [
            PreviewEvent::ChoiceAccepted {
                choice_id: accepted,
                ..
            },
            PreviewEvent::ConditionRequested(request),
        ] if accepted == &choice_id => request.clone(),
        events => panic!("expected accepted choice and pending condition, got {events:?}"),
    };
    (choice_id, request)
}

#[test]
fn accepted_choice_precedes_pending_condition_without_committing_selection() {
    let asset = branch_asset();
    let mut preview =
        PreviewSession::new(&asset, Some("start"), PreviewOptions::new()).expect("preview session");
    let (choice_id, request) = pending_branch_condition(&mut preview);
    assert!(matches!(
        preview.state().status(),
        PreviewStatus::WaitingForCondition { .. }
    ));
    assert!(
        !preview
            .session()
            .selected_choice_history()
            .iter()
            .any(|selected| selected == &choice_id)
    );
    assert!(
        preview
            .transcript()
            .events()
            .iter()
            .all(|event| !matches!(event, PreviewTranscriptEvent::ChoiceSelected { .. }))
    );

    let before = preview.session().clone();
    let failed = preview.answer(
        request.id(),
        ConditionAnswer::Failed {
            reason: "provider closed".to_owned(),
        },
        PreviewInputs::default(),
    );
    assert!(matches!(
        failed.events(),
        [
            PreviewEvent::ConditionResult { .. },
            PreviewEvent::Error(PreviewError::ConditionFailed { .. })
        ]
    ));
    assert!(
        !failed
            .events()
            .iter()
            .any(|event| matches!(event, PreviewEvent::ChoiceSelected { .. }))
    );
    assert_eq!(*preview.session(), before);
    assert!(matches!(
        preview.state().status(),
        PreviewStatus::WaitingForChoice { .. }
    ));
}

#[test]
fn accepted_choice_is_followed_by_one_committed_selection_on_success() {
    let asset = branch_asset();
    let mut preview =
        PreviewSession::new(&asset, Some("start"), PreviewOptions::new()).expect("preview session");
    let (choice_id, request) = pending_branch_condition(&mut preview);
    let completed = preview.answer(
        request.id(),
        ConditionAnswer::Value(ConditionValue::Bool(true)),
        PreviewInputs::default(),
    );
    assert!(matches!(
        completed.events().first(),
        Some(PreviewEvent::ConditionResult { .. })
    ));
    assert_eq!(
        completed
            .events()
            .iter()
            .filter(|event| matches!(event, PreviewEvent::ChoiceSelected { .. }))
            .count(),
        1
    );
    assert_eq!(
        completed
            .events()
            .iter()
            .position(|event| matches!(event, PreviewEvent::ChoiceSelected { .. })),
        Some(1)
    );
    assert_eq!(
        preview
            .session()
            .selected_choice_history()
            .iter()
            .filter(|selected| *selected == &choice_id)
            .count(),
        1
    );
    assert_eq!(
        preview
            .trace()
            .events()
            .iter()
            .filter(|event| matches!(event, PreviewEvent::ChoiceAccepted { .. }))
            .count(),
        1
    );
}
