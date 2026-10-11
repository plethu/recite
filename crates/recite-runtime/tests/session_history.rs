#[path = "support/preview.rs"]
mod support;

use recite_core::ChoiceId;
use recite_runtime::{
    DialogueEvent, EmptyDialogueContext, choose, next,
    snapshot::{encode_session_messagepack, restore_session, snapshot_session},
    start_scene,
};

#[test]
fn cloned_sessions_and_snapshots_keep_independent_histories_through_appends_and_end()
-> Result<(), Box<dyn std::error::Error>> {
    let asset = support::asset(concat!(
        ":: start default\n! deferred initial()\n",
        "? repeat@11111111111111111111\n  Repeat.\n  -> loop\n",
        "? finish@22222222222222222222\n  Finish.\n  -> finish\n",
        ":: loop\n! deferred iteration()\n",
        "? repeat@33333333333333333333\n  Repeat.\n  -> loop\n",
        "? finish@44444444444444444444\n  Finish.\n  -> finish\n",
        ":: finish\n! deferred final()\n-> END\n",
    ));
    let mut original = start_scene(&asset, None)?;
    assert!(matches!(
        next(&asset, &mut original, &EmptyDialogueContext)?,
        DialogueEvent::Prompt { .. }
    ));
    let saved = snapshot_session(&original);
    let saved_copy = saved.clone();
    let bytes = encode_session_messagepack(&original)?;
    let mut repeated = original.clone();
    let mut ending = original.clone();
    assert!(matches!(
        choose(
            &asset,
            &mut repeated,
            ChoiceId::new("11111111111111111111")?,
            &EmptyDialogueContext,
        )?,
        DialogueEvent::Prompt { .. }
    ));
    let earlier = repeated.clone();
    let earlier_snapshot = snapshot_session(&repeated);
    choose(
        &asset,
        &mut repeated,
        ChoiceId::new("33333333333333333333")?,
        &EmptyDialogueContext,
    )?;
    let DialogueEvent::End { deferred_effects } = choose(
        &asset,
        &mut ending,
        ChoiceId::new("22222222222222222222")?,
        &EmptyDialogueContext,
    )?
    else {
        panic!("ending clone must end");
    };
    assert_eq!(
        deferred_effects
            .iter()
            .map(|effect| effect.function.as_str())
            .collect::<Vec<_>>(),
        ["initial", "final"]
    );
    assert_eq!(ending.deferred_effects(), deferred_effects);
    assert_eq!(
        ending.selected_choice_history(),
        [ChoiceId::new("22222222222222222222")?]
    );
    let DialogueEvent::End { deferred_effects } = choose(
        &asset,
        &mut repeated,
        ChoiceId::new("44444444444444444444")?,
        &EmptyDialogueContext,
    )?
    else {
        panic!("repeated clone must end");
    };
    assert_eq!(
        deferred_effects
            .iter()
            .map(|effect| effect.function.as_str())
            .collect::<Vec<_>>(),
        ["initial", "iteration", "iteration", "final"]
    );
    assert_eq!(repeated.deferred_effects(), deferred_effects);
    assert_eq!(
        repeated.selected_choice_history(),
        [
            ChoiceId::new("11111111111111111111")?,
            ChoiceId::new("33333333333333333333")?,
            ChoiceId::new("44444444444444444444")?,
        ]
    );
    assert_eq!(snapshot_session(&earlier), earlier_snapshot);
    assert_eq!(snapshot_session(&original), saved);
    assert_eq!(saved, saved_copy);
    assert_eq!(encode_session_messagepack(&original)?, bytes);
    let restored = restore_session(&asset, saved)?;
    assert_eq!(encode_session_messagepack(&restored)?, bytes);
    assert_eq!(restored.selected_choice_history(), []);
    assert_eq!(restored.deferred_effects().len(), 1);
    Ok(())
}
