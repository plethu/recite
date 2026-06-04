use super::*;

#[test]
fn choice_condition_failure_keeps_session_position() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? locked requires=(trusts(player))\n",
            "    Locked.\n",
            "    -> END\n",
        ),
    );
    let failing = RecordingContext::default().failing("trusts", "condition is unavailable");
    let passing = RecordingContext::default().with("trusts", true);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_eq!(
        next_with_context(&asset, &mut session, &failing),
        Err(DialogueError::ConditionEvaluationFailed {
            function: "trusts".to_owned(),
            reason: "condition is unavailable".to_owned(),
        })
    );

    let DialogueEvent::Prompt { choices, .. } =
        next_with_context(&asset, &mut session, &passing).expect("emits prompt")
    else {
        panic!("expected prompt event");
    };
    assert_eq!(choices[0].id.as_str(), "locked");
    assert!(choices[0].availability.is_available);
}

#[test]
fn choice_conditions_mark_unavailable_choices_without_hiding_them() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? locked requires=(trusts(player))\n",
            "    Locked.\n",
            "    -> locked\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> END\n",
            ":: locked\n",
            "> locked_line\n",
            "  Locked path.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", false);
    let mut session = start_scene(&asset, None).expect("starts");

    let DialogueEvent::Prompt { choices, .. } =
        next_with_context(&asset, &mut session, &context).expect("emits prompt")
    else {
        panic!("expected prompt event");
    };
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0].id.as_str(), "locked");
    assert!(!choices[0].availability.is_available);
    assert_eq!(choices[0].availability.primary_reason, None);
    assert_eq!(choices[1].id.as_str(), "leave");
    assert!(choices[1].availability.is_available);

    let locked = ChoiceId::new("locked").expect("valid choice ID");
    assert_eq!(
        choose_with_context(&asset, &mut session, locked.clone(), &context),
        Err(DialogueError::UnavailableChoice {
            choice: locked,
            availability: Box::new(choices[0].availability.clone()),
        })
    );
    assert_eq!(
        choose_with_context(
            &asset,
            &mut session,
            ChoiceId::new("leave").expect("valid choice ID"),
            &context,
        ),
        Ok(empty_end())
    );
}

#[test]
fn unavailable_choice_exposes_primary_reason_and_reason_tree() {
    let schema = recite_core::load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .expect("valid schema fixture");
    let asset = compile_asset_with_schema(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? ask_news requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\n",
            "    Ask for private news.\n",
            "    -> END\n",
        ),
        &schema,
    );
    let context = RecordingContext::default().with("trust_gte", false);
    let mut session = start_scene(&asset, None).expect("starts");

    let DialogueEvent::Prompt { choices, .. } =
        next_with_context(&asset, &mut session, &context).expect("emits prompt")
    else {
        panic!("expected prompt event");
    };
    let availability = &choices[0].availability;
    assert!(!availability.is_available);
    assert_eq!(
        availability
            .primary_reason
            .as_ref()
            .map(|reason| (reason.id.as_str(), reason.source_text.as_str())),
        Some((
            "innkeeper_trust_hint",
            "The innkeeper is not ready to share that.",
        ))
    );

    let Some(ChoiceAvailabilityReasonTree::Reason(reason)) = &availability.reason_tree else {
        panic!("expected condition-derived reason leaf");
    };
    assert_eq!(reason.id.as_str(), "trust_too_low");
    assert_eq!(
        reason
            .args
            .iter()
            .map(|arg| (arg.name.as_str(), arg.value.as_str()))
            .collect::<Vec<_>>(),
        [("subject", "hazel"), ("target", "rhea"), ("threshold", "3"),]
    );
}

#[test]
fn available_choice_condition_can_be_selected() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? locked requires=(trusts(player))\n",
            "    Locked.\n",
            "    -> locked\n",
            ":: locked\n",
            "> locked_line\n",
            "  Locked path.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", true);
    let mut session = start_scene(&asset, None).expect("starts");

    let DialogueEvent::Prompt { choices, .. } =
        next_with_context(&asset, &mut session, &context).expect("emits prompt")
    else {
        panic!("expected prompt event");
    };
    assert!(choices[0].availability.is_available);

    assert_line(
        choose_with_context(
            &asset,
            &mut session,
            ChoiceId::new("locked").expect("valid choice ID"),
            &context,
        ),
        "locked_line",
        "Locked path.",
    );
}
