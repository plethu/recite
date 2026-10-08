use super::*;

fn pending_prompt_snapshot_for_conversion_test() -> (
    recite_core::compiled::CompiledDialogue,
    DialogueSessionSnapshot,
) {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@f58e7e11803840ed96f3\n",
            "  What next?\n",
            "  ? work@9e36f4ebaaeb53f27825\n",
            "    Work.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits prompt");
    (asset, snapshot_session(&session))
}

fn reason_snapshot(id: &str) -> DialogueChoiceAvailabilityReasonSnapshot {
    DialogueChoiceAvailabilityReasonSnapshot {
        id: id.to_owned(),
        source_text: "requires=(trust_gte(hazel, rhea, 3))".to_owned(),
        text: "hazel does not trust rhea enough (3).".to_owned(),
        origin: None,
        args: Vec::new(),
    }
}

#[test]
fn malformed_primary_reason_id_preserves_typed_conversion_source() {
    let (asset, mut snapshot) = pending_prompt_snapshot_for_conversion_test();
    snapshot
        .pending_prompt
        .as_mut()
        .expect("pending prompt")
        .choices[0]
        .availability
        .is_available = false;
    snapshot
        .pending_prompt
        .as_mut()
        .expect("pending prompt")
        .choices[0]
        .availability
        .primary_reason = Some(reason_snapshot(""));

    let error = restore_session(&asset, snapshot).expect_err("empty reason ID is invalid");
    let DialogueError::InvalidSessionSnapshot {
        reason,
        source: Some(source),
    } = &error
    else {
        panic!("expected typed invalid-session-snapshot source, got {error:?}");
    };
    assert_eq!(reason, &source.to_string());
    assert!(matches!(
        source.as_ref(),
        DialogueSessionSnapshotConversionError::InvalidAvailabilityReasonId {
            id,
            source: recite_core::CoreValueError::EmptyId {
                kind: "AvailabilityReasonId"
            },
        } if id.is_empty()
    ));
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn malformed_nested_reason_id_preserves_typed_conversion_source() {
    let (asset, mut snapshot) = pending_prompt_snapshot_for_conversion_test();
    snapshot
        .pending_prompt
        .as_mut()
        .expect("pending prompt")
        .choices[0]
        .availability
        .is_available = false;
    snapshot
        .pending_prompt
        .as_mut()
        .expect("pending prompt")
        .choices[0]
        .availability
        .reason_tree = Some(DialogueChoiceAvailabilityReasonTreeSnapshot::All(vec![
        DialogueChoiceAvailabilityReasonTreeSnapshot::Reason(reason_snapshot("  ")),
    ]));

    let error = restore_session(&asset, snapshot).expect_err("blank reason ID is invalid");
    let DialogueError::InvalidSessionSnapshot {
        reason,
        source: Some(source),
    } = &error
    else {
        panic!("expected typed invalid-session-snapshot source, got {error:?}");
    };
    assert_eq!(reason, &source.to_string());
    assert!(matches!(
        source.as_ref(),
        DialogueSessionSnapshotConversionError::InvalidAvailabilityReasonId {
            id,
            source: recite_core::CoreValueError::EmptyId {
                kind: "AvailabilityReasonId"
            },
        } if id == "  "
    ));
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn available_choice_with_reason_data_is_rejected() {
    for (primary_reason, reason_tree) in [
        (Some(reason_snapshot("requirement")), None),
        (
            None,
            Some(
                DialogueChoiceAvailabilityReasonTreeSnapshot::RequirementSourceText(
                    "forged requirement".to_owned(),
                ),
            ),
        ),
    ] {
        let (asset, mut snapshot) = pending_prompt_snapshot_for_conversion_test();
        let availability = &mut snapshot
            .pending_prompt
            .as_mut()
            .expect("pending prompt")
            .choices[0]
            .availability;
        assert!(availability.is_available);
        availability.primary_reason = primary_reason;
        availability.reason_tree = reason_tree;
        let error =
            restore_session(&asset, snapshot).expect_err("contradictory availability is invalid");
        assert!(
            matches!(error, DialogueError::InvalidSessionSnapshot { source: Some(source), .. }
            if matches!(source.as_ref(), DialogueSessionSnapshotConversionError::AvailableChoiceHasReasons))
        );
    }
}
