#[path = "support/preview.rs"]
mod support;

use std::panic::{AssertUnwindSafe, catch_unwind};

use recite_core::ChoiceId;
use recite_runtime::{
    DialogueEvent, EffectAck, EmptyDialogueContext, LocaleResolution, choose, next,
    snapshot::encode_session_messagepack, start_scene,
};

const SOURCE: &str = concat!(
    ":: start default\n! deferred initial()\n",
    "? continue@11111111111111111111\n  Continue.\n  -> later\n",
    ":: later\n! deferred second()\n! blocking wait()\n",
    "! deferred final()\n-> END\n",
);

#[test]
fn transaction_commit_matches_direct_traversal() -> Result<(), Box<dyn std::error::Error>> {
    let asset = support::asset(SOURCE);
    let mut direct = start_scene(&asset, None)?;
    let mut transactional = direct.clone();
    let direct_prompt = next(&asset, &mut direct, &EmptyDialogueContext)?;
    let prompt = transactional.try_traverse(|trial| {
        trial.next_with(&asset, &EmptyDialogueContext, LocaleResolution::new())
    })?;
    assert_eq!(prompt, direct_prompt);
    let choice = ChoiceId::new("11111111111111111111")?;
    let effect = choose(&asset, &mut direct, choice.clone(), &EmptyDialogueContext)?;
    let committed = transactional.try_traverse(|trial| {
        trial.choose_with(
            &asset,
            choice,
            &EmptyDialogueContext,
            LocaleResolution::new(),
        )
    })?;
    assert_eq!(effect, committed);
    assert_eq!(transactional, direct);
    assert_eq!(
        encode_session_messagepack(&transactional)?,
        encode_session_messagepack(&direct)?
    );
    Ok(())
}

#[test]
fn failures_and_unwinding_restore_histories_boundaries_and_retained_sessions()
-> Result<(), Box<dyn std::error::Error>> {
    let asset = support::asset(SOURCE);
    for unwind in [false, true] {
        for finish in [false, true] {
            let mut session = start_scene(&asset, None)?;
            next(&asset, &mut session, &EmptyDialogueContext)?;
            let original = session.clone();
            let before = encode_session_messagepack(&session)?;
            let mut retained_trial = None;
            let attempt = catch_unwind(AssertUnwindSafe(|| {
                session.try_traverse(|trial| {
                    let event = trial.choose_with(
                        &asset,
                        ChoiceId::new("11111111111111111111")?,
                        &EmptyDialogueContext,
                        LocaleResolution::new(),
                    )?;
                    let DialogueEvent::Effect(effect) = event else {
                        panic!("blocking effect")
                    };
                    if finish {
                        trial.acknowledge_effect(effect.id, EffectAck::Completed)?;
                        assert!(matches!(
                            trial.next_with(
                                &asset,
                                &EmptyDialogueContext,
                                LocaleResolution::new(),
                            )?,
                            DialogueEvent::End { .. }
                        ));
                    }
                    retained_trial = Some(trial.session().clone());
                    assert_ne!(trial.session(), &original);
                    if unwind {
                        panic!("output handler panicked");
                    }
                    Err::<(), Box<dyn std::error::Error>>("output rejected".into())
                })
            }));
            if unwind {
                assert!(attempt.is_err());
            } else {
                assert!(attempt.expect("returned error without unwinding").is_err());
            }
            assert_eq!(session, original);
            assert_eq!(encode_session_messagepack(&session)?, before);
            let retained = retained_trial.expect("trial session retained before rejection");
            let retained_bytes = encode_session_messagepack(&retained)?;
            let choice = ChoiceId::new("11111111111111111111")?;
            session.try_traverse(|trial| {
                trial.choose_with(
                    &asset,
                    choice,
                    &EmptyDialogueContext,
                    LocaleResolution::new(),
                )
            })?;
            assert_eq!(encode_session_messagepack(&retained)?, retained_bytes);
            assert_eq!(encode_session_messagepack(&original)?, before);
            assert_eq!(session.deferred_effects().len(), 2);
        }
    }
    Ok(())
}

#[test]
fn unique_history_rolls_back_without_a_retained_session() -> Result<(), Box<dyn std::error::Error>>
{
    let asset = support::asset(SOURCE);
    let choice = ChoiceId::new("11111111111111111111")?;
    for unwind in [false, true] {
        let mut session = start_scene(&asset, None)?;
        next(&asset, &mut session, &EmptyDialogueContext)?;
        let before = encode_session_messagepack(&session)?;
        let result = catch_unwind(AssertUnwindSafe(|| {
            session.try_traverse(|trial| {
                trial.choose_with(
                    &asset,
                    choice.clone(),
                    &EmptyDialogueContext,
                    LocaleResolution::new(),
                )?;
                if unwind {
                    panic!("rejected output");
                }
                Err::<(), recite_runtime::DialogueError>(
                    recite_runtime::DialogueError::MalformedCompiledAsset {
                        reason: "rejected output".to_owned(),
                    },
                )
            })
        }));
        assert!(result.is_err() || result.expect("non-panicking attempt").is_err());
        assert_eq!(encode_session_messagepack(&session)?, before);
        session.try_traverse(|trial| {
            trial.choose_with(
                &asset,
                choice.clone(),
                &EmptyDialogueContext,
                LocaleResolution::new(),
            )
        })?;
        assert_eq!(session.deferred_effects().len(), 2);
        assert_eq!(
            session.selected_choice_history(),
            std::slice::from_ref(&choice)
        );
    }
    Ok(())
}

#[test]
fn preview_provider_unwind_restores_the_session_and_its_history_projection()
-> Result<(), Box<dyn std::error::Error>> {
    use recite_runtime::{
        localisation::{InterpolationValueProvider, LocaleError},
        preview::{PreviewInputs, PreviewOptions, PreviewSession},
    };
    struct PanickingValues;
    impl InterpolationValueProvider for PanickingValues {
        fn lookup_value(&self, _: &str) -> Result<Option<recite_core::ScalarValue>, LocaleError> {
            panic!("host lookup panicked");
        }
    }
    let asset = support::asset(concat!(
        ":: start default\n! deferred first()\n",
        "> render@11111111111111111111 bind=(name:string=$name)\n  Hello {name}.\n-> END\n",
    ));
    let mut preview = PreviewSession::new(&asset, None, PreviewOptions::new())?;
    let before = encode_session_messagepack(preview.session())?;
    let state = preview.state().clone();
    let result = catch_unwind(AssertUnwindSafe(|| {
        preview.step(PreviewInputs::new().with_interpolation_values(&PanickingValues))
    }));
    assert!(result.is_err());
    assert_eq!(encode_session_messagepack(preview.session())?, before);
    assert_eq!(preview.state(), &state);
    assert_eq!(
        preview.state().deferred_effects(),
        preview.session().deferred_effects()
    );
    Ok(())
}

#[test]
fn rejected_traversal_restores_nested_continuations_after_reaching_end()
-> Result<(), Box<dyn std::error::Error>> {
    let asset = support::asset(concat!(
        ":: start default\n:if ready()\n  :if ready()\n",
        "    ! deferred first()\n    > inner@11111111111111111111\n      Inner.\n",
        "  > outer@22222222222222222222\n    Outer.\n",
        "> after@33333333333333333333\n  After.\n-> END\n",
    ));
    let ready = |_: recite_runtime::ConditionQuery<'_>| {
        Ok::<_, recite_runtime::ConditionEvaluationError>(recite_runtime::ConditionValue::Bool(
            true,
        ))
    };
    let mut session = start_scene(&asset, None)?;
    assert!(matches!(
        next(&asset, &mut session, &ready)?,
        DialogueEvent::Line(_)
    ));
    let before = encode_session_messagepack(&session)?;
    let rejected = session.try_traverse(|trial| {
        for _ in 0..3 {
            trial.next_with(&asset, &ready, LocaleResolution::new())?;
        }
        assert!(trial.session().is_ended());
        Err::<(), recite_runtime::DialogueError>(
            recite_runtime::DialogueError::MalformedCompiledAsset {
                reason: "reject completed output".to_owned(),
            },
        )
    });
    assert!(rejected.is_err());
    assert_eq!(encode_session_messagepack(&session)?, before);
    for expected in ["Outer.", "After."] {
        let DialogueEvent::Line(line) = next(&asset, &mut session, &ready)? else {
            panic!("expected resumed line");
        };
        assert_eq!(line.text, expected);
    }
    assert!(matches!(
        next(&asset, &mut session, &ready)?,
        DialogueEvent::End { .. }
    ));
    Ok(())
}
