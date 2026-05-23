use super::*;

#[test]
fn repeated_blocking_effect_emissions_reject_stale_acknowledgement_ids() {
    let asset = compile_asset(
        "dialogue/blocking.recite",
        concat!(
            ":: start default\n",
            "! blocking grant_item(map)\n",
            "-> start\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    let first_effect = assert_effect(
        next(&asset, &mut session),
        "grant_item",
        DialogueEffectMode::Blocking,
    );
    acknowledge_effect(&mut session, first_effect.id.clone(), EffectAck::Completed)
        .expect("first acknowledgement succeeds");

    let second_effect = assert_effect(
        next(&asset, &mut session),
        "grant_item",
        DialogueEffectMode::Blocking,
    );
    assert_ne!(first_effect.id, second_effect.id);
    assert_eq!(session.pending_effect(), Some(&second_effect));
    assert_eq!(
        acknowledge_effect(&mut session, first_effect.id.clone(), EffectAck::Completed),
        Err(DialogueError::WrongEffectAcknowledgement {
            expected: second_effect.id.clone(),
            actual: first_effect.id,
        })
    );
    assert_eq!(session.pending_effect(), Some(&second_effect));

    acknowledge_effect(&mut session, second_effect.id, EffectAck::Completed)
        .expect("second acknowledgement succeeds");
    assert!(session.pending_effect().is_none());
}

#[test]
fn blocking_effects_pause_until_acknowledged_and_resume_after_completion_or_failure() {
    let asset = compile_asset(
        "dialogue/blocking.recite",
        concat!(
            ":: start default\n",
            "> prompt\n",
            "  Choose.\n",
            "  ? work\n",
            "    Work.\n",
            "    -> work\n",
            ":: work\n",
            "! blocking grant_item(map)\n",
            "> after_grant\n",
            "  Granted.\n",
            "! blocking open_overlay(inventory)\n",
            "> after_overlay\n",
            "  Closed.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    let DialogueEvent::Prompt { .. } = next(&asset, &mut session).expect("emits prompt") else {
        panic!("expected prompt");
    };

    let first_effect = assert_effect(
        choose(
            &asset,
            &mut session,
            ChoiceId::new("work").expect("valid choice ID"),
        ),
        "grant_item",
        DialogueEffectMode::Blocking,
    );
    assert_eq!(session.pending_effect(), Some(&first_effect));
    assert_eq!(
        next(&asset, &mut session),
        Err(DialogueError::EffectPending {
            effect: first_effect.id.clone(),
        })
    );
    assert_eq!(
        choose(
            &asset,
            &mut session,
            ChoiceId::new("work").expect("valid choice ID"),
        ),
        Err(DialogueError::EffectPending {
            effect: first_effect.id.clone(),
        })
    );

    let wrong_id = EffectId::new("effect:wrong").expect("valid effect ID");
    assert_eq!(
        acknowledge_effect(&mut session, wrong_id.clone(), EffectAck::Completed),
        Err(DialogueError::WrongEffectAcknowledgement {
            expected: first_effect.id.clone(),
            actual: wrong_id,
        })
    );
    assert_eq!(session.pending_effect(), Some(&first_effect));

    acknowledge_effect(&mut session, first_effect.id.clone(), EffectAck::Completed)
        .expect("completed acknowledgement succeeds");
    assert!(session.pending_effect().is_none());
    assert_line(next(&asset, &mut session), "after_grant", "Granted.");

    let second_effect = assert_effect(
        next(&asset, &mut session),
        "open_overlay",
        DialogueEffectMode::Blocking,
    );
    acknowledge_effect(
        &mut session,
        second_effect.id.clone(),
        EffectAck::Failed {
            reason: "overlay closed externally".to_owned(),
        },
    )
    .expect("failed acknowledgement succeeds");
    assert!(session.pending_effect().is_none());
    assert_eq!(
        acknowledge_effect(&mut session, second_effect.id.clone(), EffectAck::Completed),
        Err(DialogueError::NoEffectPending {
            effect: second_effect.id,
        })
    );
    assert_line(next(&asset, &mut session), "after_overlay", "Closed.");
    assert_end_effects(next(&asset, &mut session), []);
}
