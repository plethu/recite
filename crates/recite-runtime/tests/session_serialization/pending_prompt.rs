use super::*;

#[test]
fn restores_pending_prompt_and_selects_choice_using_matching_asset() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@87262b29c62cbd5e039d\n",
            "  What next?\n",
            "  ? ask_work@9c9d3a385f3d2f73d105\n",
            "    Ask about work.\n",
            "    -> work\n",
            "  ? leave@49bf893f86107eba6990\n",
            "    Leave.\n",
            "    -> END\n",
            ":: work\n",
            "> work_line@a248167d24afe69ed5ed\n",
            "  Work waits.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    let DialogueEvent::Prompt { choices, .. } = next(&asset, &mut session).expect("emits prompt")
    else {
        panic!("expected prompt");
    };
    assert_eq!(
        choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        ["9c9d3a385f3d2f73d105", "49bf893f86107eba6990"]
    );

    let snapshot = snapshot_session(&session);
    assert_eq!(
        snapshot
            .pending_prompt
            .as_ref()
            .expect("pending prompt is serialized")
            .choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        ["9c9d3a385f3d2f73d105", "49bf893f86107eba6990"]
    );

    let mut restored = restore_session(&asset, snapshot).expect("restores pending prompt");
    assert_line(
        choose(
            &asset,
            &mut restored,
            ChoiceId::new("9c9d3a385f3d2f73d105").expect("valid choice ID"),
        ),
        "a248167d24afe69ed5ed",
        "Work waits.",
    );
}

#[test]
fn restores_pending_prompt_choice_availability_reasons() {
    let schema = recite_core::load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .expect("valid schema fixture");
    let asset = compile_asset_with_schema(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@b3e7889a570fec166769\n",
            "  What next?\n",
            "  ? ask_news@705a9c980adc29875933 requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\n",
            "    Ask for private news.\n",
            "    -> END\n",
        ),
        &schema,
    );
    let context = |query: recite_runtime::ConditionQuery<'_>| {
        assert_eq!(query.function(), "trust_gte");
        Ok(recite_runtime::ConditionValue::Bool(false))
    };
    let mut session = start_scene(&asset, None).expect("starts");
    let DialogueEvent::Prompt { choices, .. } =
        runtime_next(&asset, &mut session, &context).expect("emits prompt")
    else {
        panic!("expected prompt");
    };
    assert!(!choices[0].availability.is_available);

    let snapshot = snapshot_session(&session);
    assert_eq!(
        snapshot.pending_prompt.as_ref().expect("pending").choices[0]
            .availability
            .primary_reason
            .as_ref()
            .map(|reason| reason.id.as_str()),
        Some("innkeeper_trust_hint")
    );

    let restored = restore_session(&asset, snapshot).expect("restores pending prompt");
    let restored_snapshot = snapshot_session(&restored);
    let restored_choice = &restored_snapshot
        .pending_prompt
        .as_ref()
        .expect("pending prompt")
        .choices[0];
    assert_eq!(
        restored_choice
            .availability
            .primary_reason
            .as_ref()
            .map(|reason| reason.id.as_str()),
        Some("innkeeper_trust_hint")
    );
    assert_eq!(
        restored_choice
            .availability
            .primary_reason
            .as_ref()
            .and_then(|reason| reason.origin.as_ref()),
        Some(
            &DialogueChoiceAvailabilityReasonOriginSnapshot::RequirementExpression {
                source_text: "requires=(trust_gte(hazel, rhea, 3))".to_owned(),
            }
        )
    );
    let Some(DialogueChoiceAvailabilityReasonTreeSnapshot::Reason(reason)) =
        &restored_choice.availability.reason_tree
    else {
        panic!("expected reason tree");
    };
    assert_eq!(reason.id.as_str(), "trust_too_low");
    assert_eq!(reason.text, "hazel does not trust rhea enough (3).");
    assert_eq!(
        reason.origin,
        Some(
            DialogueChoiceAvailabilityReasonOriginSnapshot::ConditionCall {
                function: "trust_gte".to_owned(),
                args: vec![
                    DialogueChoiceAvailabilityReasonValueSnapshot::Identifier("hazel".to_owned()),
                    DialogueChoiceAvailabilityReasonValueSnapshot::Identifier("rhea".to_owned()),
                    DialogueChoiceAvailabilityReasonValueSnapshot::Integer(3),
                ],
            }
        )
    );
}

#[test]
fn restores_selected_choice_history_after_choice_continuation() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@85daba18ac6920654eb8\n",
            "  What next?\n",
            "  ? work@b2135e46d6825d504149\n",
            "    Work.\n",
            "    -> work\n",
            ":: work\n",
            "> work_line@d100aa1a09e300b0f35d\n",
            "  Work waits.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits prompt");
    assert_line(
        choose(
            &asset,
            &mut session,
            ChoiceId::new("b2135e46d6825d504149").expect("valid choice ID"),
        ),
        "d100aa1a09e300b0f35d",
        "Work waits.",
    );

    let mut restored =
        restore_session(&asset, snapshot_session(&session)).expect("restores after choice");
    assert_eq!(
        restored
            .selected_choice_history()
            .iter()
            .map(ChoiceId::as_str)
            .collect::<Vec<_>>(),
        ["b2135e46d6825d504149"]
    );
    assert_eq!(next(&asset, &mut restored), Ok(empty_end()));
}
