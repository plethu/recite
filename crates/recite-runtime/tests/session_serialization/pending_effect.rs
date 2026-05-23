use super::*;

#[test]
fn restores_pending_effect_and_reemits_same_request_once() {
    let asset = blocking_asset();
    let mut session = start_scene(&asset, None).expect("starts");
    let effect = assert_effect(next(&asset, &mut session), "grant_item");
    let snapshot = snapshot_session(&session);

    assert_eq!(
        snapshot
            .pending_effect
            .as_ref()
            .expect("pending effect is serialized")
            .id,
        effect.id.as_str()
    );

    let mut restored = restore_session(&asset, snapshot).expect("restores pending effect");
    assert_eq!(restored.pending_effect(), Some(&effect));
    let before_reemit = snapshot_session(&restored);

    let reemitted = assert_effect(next(&asset, &mut restored), "grant_item");
    assert_eq!(reemitted, effect);
    let after_reemit = snapshot_session(&restored);
    assert_eq!(after_reemit.trace_counter, before_reemit.trace_counter);
    assert_eq!(after_reemit.next_statement, before_reemit.next_statement);
    assert_eq!(
        next(&asset, &mut restored),
        Err(DialogueError::EffectPending {
            effect: effect.id.clone(),
        })
    );
}

#[test]
fn messagepack_round_trip_restores_pending_effect_and_reemits_same_request() {
    let asset = blocking_asset();
    let mut session = start_scene(&asset, None).expect("starts");
    let effect = assert_effect(next(&asset, &mut session), "grant_item");
    let bytes = encode_session_messagepack(&session).expect("encodes session");

    let mut restored = decode_session_messagepack(&asset, &bytes).expect("decodes blocked session");
    assert_eq!(restored.pending_effect(), Some(&effect));
    assert_eq!(
        assert_effect(next(&asset, &mut restored), "grant_item"),
        effect
    );
}

#[test]
fn completed_acknowledgement_after_restore_resumes_traversal() {
    let asset = blocking_asset();
    let mut session = start_scene(&asset, None).expect("starts");
    let effect = assert_effect(next(&asset, &mut session), "grant_item");

    let mut restored =
        restore_session(&asset, snapshot_session(&session)).expect("restores pending effect");
    acknowledge_effect(&mut restored, effect.id, EffectAck::Completed)
        .expect("acknowledgement succeeds");

    assert_line(next(&asset, &mut restored), "after_grant", "Granted.");
    assert_end_effects(next(&asset, &mut restored), []);
}

#[test]
fn failed_acknowledgement_after_restore_resumes_traversal() {
    let asset = blocking_asset();
    let mut session = start_scene(&asset, None).expect("starts");
    let effect = assert_effect(next(&asset, &mut session), "grant_item");

    let mut restored =
        restore_session(&asset, snapshot_session(&session)).expect("restores pending effect");
    acknowledge_effect(
        &mut restored,
        effect.id,
        EffectAck::Failed {
            reason: "host reconciled after load".to_owned(),
        },
    )
    .expect("acknowledgement succeeds");

    assert_line(next(&asset, &mut restored), "after_grant", "Granted.");
    assert_end_effects(next(&asset, &mut restored), []);
}

#[test]
fn wrong_acknowledgement_after_restore_keeps_pending_effect_blocked() {
    let asset = blocking_asset();
    let mut session = start_scene(&asset, None).expect("starts");
    let effect = assert_effect(next(&asset, &mut session), "grant_item");
    let mut restored =
        restore_session(&asset, snapshot_session(&session)).expect("restores pending effect");

    let wrong_id = EffectId::new("effect:wrong#1").expect("valid effect ID");
    assert_eq!(
        acknowledge_effect(&mut restored, wrong_id.clone(), EffectAck::Completed),
        Err(DialogueError::WrongEffectAcknowledgement {
            expected: effect.id.clone(),
            actual: wrong_id,
        })
    );
    assert_eq!(restored.pending_effect(), Some(&effect));
    assert_eq!(
        assert_effect(next(&asset, &mut restored), "grant_item"),
        effect.clone()
    );
    assert_eq!(
        next(&asset, &mut restored),
        Err(DialogueError::EffectPending { effect: effect.id })
    );
}

fn blocking_asset() -> CompiledDialogue {
    compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! blocking grant_item(map)\n",
            "> after_grant\n",
            "  Granted.\n",
            "-> END\n",
        ),
    )
}
