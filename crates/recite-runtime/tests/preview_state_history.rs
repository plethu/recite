#[path = "support/preview.rs"]
mod preview_support;

use recite_core::ChoiceId;
use recite_runtime::{
    ConditionValue,
    preview::{
        ConditionAnswer, PreviewCommand, PreviewEvent, PreviewInputs, PreviewOptions,
        PreviewSession,
    },
};

#[test]
fn earlier_preview_outputs_and_snapshots_survive_appends_restart_and_restore()
-> Result<(), Box<dyn std::error::Error>> {
    let asset = preview_support::asset(include_str!(
        "../../../fixtures/recite/valid/benchmarks/session_deferred.recite"
    ));
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new())?;
    let opening = preview.step(PreviewInputs::new());
    let request = opening
        .events()
        .iter()
        .find_map(|event| match event {
            PreviewEvent::ConditionRequested(request) => Some(request.id()),
            _ => None,
        })
        .ok_or("missing opening condition")?;
    let first = preview.answer(
        request,
        ConditionAnswer::Value(ConditionValue::Bool(true)),
        PreviewInputs::new(),
    );
    let snapshot = preview.snapshot()?;
    let encoded = snapshot.encode()?;
    assert!(first.state().selected_choice_history().is_empty());
    assert_eq!(first.state().deferred_effects().len(), 1);

    let selecting = preview.choose(ChoiceId::new("11111111111111111111")?, PreviewInputs::new());
    let request = selecting
        .events()
        .iter()
        .find_map(|event| match event {
            PreviewEvent::ConditionRequested(request) => Some(request.id()),
            _ => None,
        })
        .ok_or("missing follow-up condition")?;
    assert!(
        preview.session().selected_choice_history().is_empty(),
        "suspended selection is not committed"
    );
    let second = preview.answer(
        request,
        ConditionAnswer::Value(ConditionValue::Bool(true)),
        PreviewInputs::new(),
    );
    assert_eq!(second.state().selected_choice_history().len(), 1);
    assert_eq!(second.state().deferred_effects().len(), 2);

    preview.dispatch(PreviewCommand::Restart, PreviewInputs::new());
    assert!(preview.state().selected_choice_history().is_empty());
    assert!(preview.state().deferred_effects().is_empty());
    preview.restore(snapshot.clone())?;
    assert_eq!(preview.state(), first.state());
    assert_eq!(snapshot.encode()?, encoded);
    assert!(first.state().selected_choice_history().is_empty());
    assert_eq!(first.state().deferred_effects().len(), 1);
    assert_eq!(second.state().selected_choice_history().len(), 1);
    assert_eq!(second.state().deferred_effects().len(), 2);
    Ok(())
}

#[test]
fn deferred_effects_before_suspension_commit_once_and_failed_replay_discards_them()
-> Result<(), Box<dyn std::error::Error>> {
    let asset = preview_support::asset(concat!(
        ":: start default\n! deferred progress()\n",
        ":if ready()\n  ! deferred second()\n  :if ready()\n",
        "    ? continue@11111111111111111111\n      Continue.\n      -> start\n",
        "  :else\n    -> END\n:else\n  -> END\n",
    ));
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new())?;
    let initial = preview.session().clone();
    let first = requested(&preview.step(PreviewInputs::new()));
    let wrong = preview.answer(
        first,
        ConditionAnswer::Value(ConditionValue::EnumVariant("wrong".to_owned())),
        PreviewInputs::new(),
    );
    assert!(matches!(wrong.events(), [PreviewEvent::Error(_)]));
    assert_eq!(preview.session(), &initial);
    let second = requested(&preview.answer(
        first,
        ConditionAnswer::Value(ConditionValue::Bool(true)),
        PreviewInputs::new(),
    ));
    assert_eq!(preview.session(), &initial);
    let failed = preview.answer(
        second,
        ConditionAnswer::Failed {
            reason: "host refused".to_owned(),
        },
        PreviewInputs::new(),
    );
    assert!(
        failed
            .events()
            .iter()
            .any(|event| matches!(event, PreviewEvent::Error(_)))
    );
    assert_eq!(preview.session(), &initial);
    assert!(preview.state().deferred_effects().is_empty());

    let first = requested(&preview.step(PreviewInputs::new()));
    let second = requested(&preview.answer(
        first,
        ConditionAnswer::Value(ConditionValue::Bool(true)),
        PreviewInputs::new(),
    ));
    let completed = preview.answer(
        second,
        ConditionAnswer::Value(ConditionValue::Bool(true)),
        PreviewInputs::new(),
    );
    assert_eq!(preview.session().deferred_effects().len(), 2);
    assert_eq!(
        completed.state().deferred_effects(),
        preview.session().deferred_effects()
    );
    assert_eq!(
        completed
            .events()
            .iter()
            .filter(|event| matches!(event, PreviewEvent::DeferredEffectScheduled(_)))
            .count(),
        2
    );
    Ok(())
}

fn requested(
    output: &recite_runtime::preview::PreviewOutput,
) -> recite_runtime::preview::PreviewConditionRequestId {
    match output.events().iter().find_map(|event| match event {
        PreviewEvent::ConditionRequested(request) => Some(request.id()),
        _ => None,
    }) {
        Some(id) => id,
        None => panic!("expected pending condition: {:?}", output.events()),
    }
}

#[test]
fn rejected_and_failed_answers_preserve_nonempty_history_for_retry_and_restore()
-> Result<(), Box<dyn std::error::Error>> {
    let asset = preview_support::asset(include_str!(
        "../../../fixtures/recite/valid/benchmarks/session_deferred.recite"
    ));
    let choice = ChoiceId::new("11111111111111111111")?;
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new())?;
    let opening = requested(&preview.step(PreviewInputs::new()));
    preview.answer(
        opening,
        ConditionAnswer::Value(ConditionValue::Bool(true)),
        PreviewInputs::new(),
    );
    for _ in 0..4 {
        let pending = requested(&preview.choose(choice.clone(), PreviewInputs::new()));
        preview.answer(
            pending,
            ConditionAnswer::Value(ConditionValue::Bool(true)),
            PreviewInputs::new(),
        );
    }
    let before = preview.session().clone();
    let saved = preview.snapshot()?;
    let saved_copy = saved.clone();
    let bytes = saved.encode()?;
    let pending = requested(&preview.choose(choice.clone(), PreviewInputs::new()));
    let rejected = preview.answer(
        pending,
        ConditionAnswer::Value(ConditionValue::EnumVariant("wrong".to_owned())),
        PreviewInputs::new(),
    );
    assert!(matches!(rejected.events(), [PreviewEvent::Error(_)]));
    assert_eq!(preview.session(), &before);
    let failed = preview.answer(
        pending,
        ConditionAnswer::Failed {
            reason: "retry later".to_owned(),
        },
        PreviewInputs::new(),
    );
    assert!(matches!(
        failed.events(),
        [PreviewEvent::ConditionResult { .. }, PreviewEvent::Error(_)]
    ));
    assert_eq!(preview.session(), &before);
    assert_eq!(
        failed.state().selected_choice_history(),
        before.selected_choice_history()
    );
    assert_eq!(failed.state().deferred_effects(), before.deferred_effects());
    let pending = requested(&preview.choose(choice, PreviewInputs::new()));
    let completed = preview.answer(
        pending,
        ConditionAnswer::Value(ConditionValue::Bool(true)),
        PreviewInputs::new(),
    );
    assert_eq!(completed.state().selected_choice_history().len(), 5);
    assert_eq!(completed.state().deferred_effects().len(), 6);
    assert_eq!(
        completed
            .events()
            .iter()
            .filter(|event| matches!(event, PreviewEvent::DeferredEffectScheduled(_)))
            .count(),
        1
    );
    assert_eq!(saved, saved_copy);
    assert_eq!(saved.encode()?, bytes);
    assert_eq!(before.selected_choice_history().len(), 4);
    assert_eq!(before.deferred_effects().len(), 5);
    preview.restore(saved)?;
    assert_eq!(preview.session(), &before);
    assert_eq!(completed.state().selected_choice_history().len(), 5);
    assert_eq!(completed.state().deferred_effects().len(), 6);
    Ok(())
}
