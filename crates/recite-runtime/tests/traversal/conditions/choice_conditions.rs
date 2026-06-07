use super::*;

#[test]
fn choice_condition_failure_keeps_session_position() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@5f332cf0725195edb2b8\n",
            "  What next?\n",
            "  ? locked@baa4233d710cc23fd277 requires=(trusts(player))\n",
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
    assert_eq!(choices[0].id.as_str(), "baa4233d710cc23fd277");
    assert!(choices[0].availability.is_available);
}

#[test]
fn choice_conditions_mark_unavailable_choices_without_hiding_them() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@119199f50c84d430fd79\n",
            "  What next?\n",
            "  ? locked@dff55b26a02abe254920 requires=(trusts(player))\n",
            "    Locked.\n",
            "    -> locked\n",
            "  ? leave@630ece67467f4cc775ca\n",
            "    Leave.\n",
            "    -> END\n",
            ":: locked\n",
            "> locked_line@40da158abeea3c3a48e9\n",
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
    assert_eq!(choices[0].id.as_str(), "dff55b26a02abe254920");
    assert!(!choices[0].availability.is_available);
    assert_eq!(choices[0].availability.primary_reason, None);
    assert_eq!(choices[1].id.as_str(), "630ece67467f4cc775ca");
    assert!(choices[1].availability.is_available);

    let locked = ChoiceId::new("dff55b26a02abe254920").expect("valid choice ID");
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
            ChoiceId::new("630ece67467f4cc775ca").expect("valid choice ID"),
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
            "> prompt_line@b725fc1b0ab00561ac37\n",
            "  What next?\n",
            "  ? ask_news@c1db3914d2910b958050 requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\n",
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
    let mut schema = recite_core::load_schema_manifest_str(
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
            "> prompt_line@d77130f2adfaa90945b7\n",
            "  What next?\n",
            "  ? ask_news@951da0aaf4d0caab698d requires=(has_key(hazel, rhea, 1) and trust_gte(hazel, rhea, 3))\n",
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
    let mut schema = recite_core::load_schema_manifest_str(
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
            "> prompt_line@aae712ebc576b27b3235\n",
            "  What next?\n",
            "  ? ask_news@d213df7e71995f5c202c requires=(trust_gte(hazel, rhea, 3) or missing_condition(hazel, rhea, 1))\n",
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
            "> prompt_line@9f1ff626175374f7b98d\n",
            "  What next?\n",
            "  ? ask_news@b8af63c339a222bdf3b9 requires=(not trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\n",
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
            "> prompt_line@f16655c71a1d49cc2ce3\n",
            "  What next?\n",
            "  ? locked@6015d3fa8bf1508ea9e3 requires=(trusts(player))\n",
            "    Locked.\n",
            "    -> locked\n",
            ":: locked\n",
            "> locked_line@42e4b38b5929000271fa\n",
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
            ChoiceId::new("6015d3fa8bf1508ea9e3").expect("valid choice ID"),
            &context,
        ),
        "42e4b38b5929000271fa",
        "Locked path.",
    );
}
