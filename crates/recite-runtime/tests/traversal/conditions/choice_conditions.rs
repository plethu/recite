use super::*;

#[test]
fn choice_condition_failure_keeps_session_position() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @w3nkrtt\n",
            "  What next?\n",
            "  ? requires=(trusts(player)) @sr27cfr\n",
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
            kind: recite_runtime::ConditionEvaluationErrorKind::EvaluationFailed,
        })
    );

    let DialogueEvent::Prompt { choices, .. } =
        next_with_context(&asset, &mut session, &passing).expect("emits prompt")
    else {
        panic!("expected prompt event");
    };
    assert_eq!(choices[0].id.as_str(), "sr27cfr");
    assert!(choices[0].availability.is_available);
}

#[test]
fn choice_conditions_mark_unavailable_choices_without_hiding_them() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @hd0v2q1\n",
            "  What next?\n",
            "  ? requires=(trusts(player)) @j4td3ks\n",
            "    Locked.\n",
            "    -> locked\n",
            "  ? @leave\n",
            "    Leave.\n",
            "    -> END\n",
            ":: locked\n",
            "> @q3vfrbj\n",
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
    assert_eq!(choices[0].id.as_str(), "j4td3ks");
    assert!(!choices[0].availability.is_available);
    assert_eq!(choices[0].availability.primary_reason, None);
    assert_eq!(choices[1].id.as_str(), "leave");
    assert!(choices[1].availability.is_available);

    let locked = ChoiceId::new("j4td3ks").expect("valid choice ID");
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
    let schema = recite_core::schema::load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .expect("valid schema fixture");
    let asset = compile_asset_with_schema(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @mbhhdxx\n",
            "  What next?\n",
            "  ? requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint @zse4w3f\n",
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
        availability.primary_reason.as_ref().map(|reason| {
            (
                reason.id.as_str(),
                reason.source_text.as_str(),
                reason.text.as_str(),
            )
        }),
        Some((
            "innkeeper_trust_hint",
            "The innkeeper is not ready to share that.",
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
            .map(|arg| (arg.name.as_str(), &arg.value))
            .collect::<Vec<_>>(),
        [
            (
                "subject",
                &ChoiceAvailabilityReasonValue::Identifier("hazel".to_owned())
            ),
            (
                "target",
                &ChoiceAvailabilityReasonValue::Identifier("rhea".to_owned())
            ),
            ("threshold", &ChoiceAvailabilityReasonValue::Integer(3)),
        ]
    );
    assert_eq!(
        reason.origin,
        Some(ChoiceAvailabilityReasonOrigin::ConditionCall {
            function: "trust_gte".to_owned(),
            args: vec![
                ChoiceAvailabilityReasonValue::Identifier("hazel".to_owned()),
                ChoiceAvailabilityReasonValue::Identifier("rhea".to_owned()),
                ChoiceAvailabilityReasonValue::Integer(3),
            ],
        })
    );
    assert_eq!(
        availability
            .primary_reason
            .as_ref()
            .and_then(|reason| reason.origin.as_ref()),
        Some(&ChoiceAvailabilityReasonOrigin::RequirementExpression {
            source_text: "requires=(trust_gte(hazel, rhea, 3))".to_owned(),
        })
    );
}

#[test]
fn and_reason_tree_contains_only_failed_children() {
    let mut schema = recite_core::schema::load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .expect("valid schema fixture");
    let trust_gte = schema
        .conditions
        .get("trust_gte")
        .expect("schema has trust condition")
        .clone();
    schema.conditions.insert("has_key".to_owned(), trust_gte);
    let asset = compile_asset_with_schema(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @hmcvgk7\n",
            "  What next?\n",
            "  ? requires=(has_key(hazel, rhea, 1) and trust_gte(hazel, rhea, 3)) @ntzcrng\n",
            "    Ask for private news.\n",
            "    -> END\n",
        ),
        &schema,
    );
    let context = RecordingContext::default()
        .with("has_key", true)
        .with("trust_gte", false);
    let mut session = start_scene(&asset, None).expect("starts");

    let DialogueEvent::Prompt { choices, .. } =
        next_with_context(&asset, &mut session, &context).expect("emits prompt")
    else {
        panic!("expected prompt event");
    };

    let Some(ChoiceAvailabilityReasonTree::All(children)) = &choices[0].availability.reason_tree
    else {
        panic!("expected all reason tree");
    };
    assert_eq!(children.len(), 1);
    let ChoiceAvailabilityReasonTree::Reason(reason) = &children[0] else {
        panic!("expected failed condition reason");
    };
    assert_eq!(reason.id.as_str(), "trust_too_low");
    assert_eq!(
        reason.origin,
        Some(ChoiceAvailabilityReasonOrigin::ConditionCall {
            function: "trust_gte".to_owned(),
            args: vec![
                ChoiceAvailabilityReasonValue::Identifier("hazel".to_owned()),
                ChoiceAvailabilityReasonValue::Identifier("rhea".to_owned()),
                ChoiceAvailabilityReasonValue::Integer(3),
            ],
        })
    );
}

#[test]
fn or_requirement_short_circuits_after_passing_child() {
    let mut schema = recite_core::schema::load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .expect("valid schema fixture");
    let trust_gte = schema
        .conditions
        .get("trust_gte")
        .expect("schema has trust condition")
        .clone();
    schema
        .conditions
        .insert("missing_condition".to_owned(), trust_gte);
    let asset = compile_asset_with_schema(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @gk5110t\n",
            "  What next?\n",
            "  ? requires=(trust_gte(hazel, rhea, 3) or missing_condition(hazel, rhea, 1)) @dzpb3p2\n",
            "    Ask for private news.\n",
            "    -> END\n",
        ),
        &schema,
    );
    let context = RecordingContext::default().with("trust_gte", true);
    let mut session = start_scene(&asset, None).expect("starts");

    let DialogueEvent::Prompt { choices, .. } =
        next_with_context(&asset, &mut session, &context).expect("emits prompt")
    else {
        panic!("expected prompt event");
    };

    assert!(choices[0].availability.is_available);
    assert_eq!(
        context
            .calls()
            .into_iter()
            .map(|call| call.function)
            .collect::<Vec<_>>(),
        ["trust_gte"]
    );
}

#[test]
fn negated_requirement_does_not_synthesize_automatic_reason_tree() {
    let schema = recite_core::schema::load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../../../fixtures/schema/valid/generated_manifest.json"),
    )
    .schema
    .expect("valid schema fixture");
    let asset = compile_asset_with_schema(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @hpnzcrh\n",
            "  What next?\n",
            "  ? requires=(not trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint @xrhr78n\n",
            "    Ask for private news.\n",
            "    -> END\n",
        ),
        &schema,
    );
    let context = RecordingContext::default().with("trust_gte", true);
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
            .map(|reason| reason.id.as_str()),
        Some("innkeeper_trust_hint")
    );
    assert_eq!(availability.reason_tree, None);
}

#[test]
fn available_choice_condition_can_be_selected() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @bja37kc\n",
            "  What next?\n",
            "  ? requires=(trusts(player)) @qw22b06\n",
            "    Locked.\n",
            "    -> locked\n",
            ":: locked\n",
            "> @sn4q11k\n",
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
            ChoiceId::new("qw22b06").expect("valid choice ID"),
            &context,
        ),
        "sn4q11k",
        "Locked path.",
    );
}
