use super::*;

#[test]
fn lowering_parses_statement_vocabulary_and_conditions() {
    let source = concat!(
        ":: tavern_arrival default speaker=innkeeper scene=opening scene=repeat\n",
        "# scene opener\n",
        "> prompt@5e0925cd041f2f6df9e2 speaker=innkeeper portrait=neutral sfx=door sfx=mug\n",
        "  What do you need?\n",
        "\n",
        "  ? ask_news@8d398d18dbde0d7303c2 echo=selected_text sfx=paper requires=(familiarity_gte(hazel, rhea, 3))\n",
        "    What's the news?\n",
        "    -> local_news\n",
        ":if not thread_completed(rhea_job_response) and familiarity_gte(hazel, rhea, 3)\n",
        "  > gated_line@f6bbac5df45570bc709f\n",
        "    Still waiting.\n",
        ":else\n",
        "  > fallback_line@cc27810a0d2aeed3edea\n",
        "    Fine.\n",
        "! deferred advance_thread(rhea_job_response, tired)\n",
        ":match thread_stage(rhea_job_response)\n",
        "  :case tired\n",
        "    > tired_line@05283c0e9a1365072e3f\n",
        "      I'm tired.\n",
        "  :case _\n",
        "    ! immediate play_sfx(snap)\n",
        "-> END\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let block = single_block(&lowered);
    assert!(block.is_default);
    assert_eq!(
        block.default_speaker.as_ref().map(SpeakerId::as_str),
        Some("innkeeper")
    );
    assert_eq!(
        block
            .metadata
            .iter()
            .map(|entry| entry.key.as_str())
            .collect::<Vec<_>>(),
        ["scene", "scene"]
    );
    assert_eq!(
        block
            .statements
            .iter()
            .map(Statement::kind)
            .collect::<Vec<_>>(),
        [
            StatementKind::Comment,
            StatementKind::Line,
            StatementKind::If,
            StatementKind::Effect,
            StatementKind::Match,
            StatementKind::Divert,
        ]
    );

    let prompt = line_statement(block, 1);
    assert_eq!(prompt.source_text.text, "What do you need?");
    assert_eq!(
        prompt
            .metadata
            .iter()
            .map(|entry| entry.key.as_str())
            .collect::<Vec<_>>(),
        ["portrait", "sfx", "sfx"]
    );

    let choice = nested_choice(prompt, 0);
    assert_eq!(
        choice.id.as_ref().map(recite_core::ChoiceId::as_str),
        Some("8d398d18dbde0d7303c2")
    );
    assert_eq!(choice.echo, ChoiceEcho::SelectedText);
    assert_eq!(choice.source_text.text, "What's the news?");
    assert!(choice.availability_requirement.is_some());
    assert_eq!(
        choice.target.as_ref().map(|target| &target.target),
        Some(&DivertTarget::Block(recite_core::BlockReference::local(
            recite_core::BlockId::new("local_news").expect("valid block id")
        )))
    );
    assert_eq!(
        choice
            .target
            .as_ref()
            .map(|target| target.span.start.line()),
        Some(8)
    );

    let branch = if_statement(block, 2);
    let ConditionExpression::And(group) = &branch.condition else {
        panic!("expected top-level and condition");
    };
    assert_eq!(group.expressions.len(), 2);
    assert!(matches!(group.expressions[0], ConditionExpression::Not(_)));
    assert_eq!(branch.then_statements.len(), 1);
    assert_eq!(branch.else_statements.len(), 1);

    let Statement::Effect(effect) = &block.statements[3] else {
        panic!("expected effect");
    };
    assert_eq!(effect.mode, EffectMode::Deferred);
    assert_eq!(effect.function, "advance_thread");
    assert_eq!(
        effect.args,
        [
            Argument::identifier("rhea_job_response"),
            Argument::identifier("tired")
        ]
    );

    let match_branch = match_statement(block, 4);
    assert_eq!(match_branch.scrutinee.function, "thread_stage");
    assert_eq!(match_branch.arms.len(), 2);
    assert_eq!(
        match_branch.arms[0].pattern,
        MatchPattern::Variant("tired".to_owned())
    );
    assert_eq!(match_branch.arms[1].pattern, MatchPattern::Wildcard);
}

#[test]
fn choice_requires_and_reason_clauses_are_not_metadata() {
    let source = concat!(
        ":: tavern_arrival\n",
        "? ask_news@bc0a5874483fd8a329fd sfx=paper requires=(trust_gte(hazel, rhea, 3)) topic=rumours reason=innkeeper_trust_hint\n",
        "  What's the news?\n",
        "  -> END\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let choice = choice_statement(single_block(&lowered), 0);
    assert_eq!(
        choice
            .metadata
            .iter()
            .map(|entry| entry.key.as_str())
            .collect::<Vec<_>>(),
        ["sfx", "topic"]
    );
    let requirement = choice
        .availability_requirement
        .as_ref()
        .expect("requires clause lowers");
    let ConditionExpression::Grouped(grouped) = &requirement.condition else {
        panic!("expected requires value to preserve grouped expression");
    };
    let ConditionExpression::Call(call) = grouped.expression.as_ref() else {
        panic!("expected condition call");
    };
    assert_eq!(call.function, "trust_gte");
    assert_eq!(
        call.args,
        [
            Argument::identifier("hazel"),
            Argument::identifier("rhea"),
            ScalarValue::from(3_i64).into(),
        ]
    );
    let reason = choice
        .availability_reason_override
        .as_ref()
        .expect("reason clause lowers");
    assert_eq!(reason.reason_id.as_str(), "innkeeper_trust_hint");
    assert!(reason.argument_span.is_none());
}

#[test]
fn bare_reason_without_requires_is_representable_before_validation() {
    let source = concat!(
        ":: tavern_arrival\n",
        "? ask_news@af1afd5f236b8e93d917 reason=innkeeper_trust_hint\n",
        "  What's the news?\n",
        "  -> END\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let choice = choice_statement(single_block(&lowered), 0);
    assert!(choice.availability_requirement.is_none());
    assert_eq!(
        choice
            .availability_reason_override
            .as_ref()
            .map(|reason| reason.reason_id.as_str()),
        Some("innkeeper_trust_hint")
    );
}

#[test]
fn parameterized_reason_clause_preserves_argument_span_for_validation() {
    let source = concat!(
        ":: tavern_arrival\n",
        "? ask_news@7bbdd4c4c2b97516923d requires=(trust_gte(hazel, rhea, 3)) reason=trust_too_low(hazel)\n",
        "  What's the news?\n",
        "  -> END\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let choice = choice_statement(single_block(&lowered), 0);
    let reason = choice
        .availability_reason_override
        .as_ref()
        .expect("reason clause lowers");
    assert_eq!(reason.reason_id.as_str(), "trust_too_low");
    assert_eq!(
        reason
            .argument_span
            .as_ref()
            .map(|span| (span.start.line(), span.start.column())),
        Some((2, 90))
    );
}

#[test]
fn malformed_choice_availability_clauses_report_diagnostics() {
    let source = concat!(
        ":: tavern_arrival\n",
        "? ask_news@43427ec20eb491b44bbb requires=(trust_gte(\n",
        "  What's the news?\n",
        "? ask_more@70a460e3cecd373440d5 reason=trust_too_low(\n",
        "  More?\n",
        "? ask_again@047176c7b6a98aa21676 requires=(trust_gte(hazel, rhea, 3)) requires=(trust_gte(hazel, rhea, 4)) reason=innkeeper_trust_hint reason=other\n",
        "  Again?\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(
        &lowered,
        [
            "RECITE_PARSE013",
            "RECITE_PARSE008",
            "RECITE_PARSE008",
            "RECITE_PARSE008",
        ],
    );
}

#[test]
fn trailing_choice_if_reports_targeted_help_without_condition_cascade() {
    let source = concat!(
        ":: tavern_arrival\n",
        "? ask_news@a74221348f0e47548c59 if trust_gte(hazel, rhea, 3)\n",
        "  What's the news?\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(&lowered, ["RECITE_PARSE018"]);
    assert_eq!(
        lowered.diagnostics[0].help.as_deref(),
        Some("use requires=(...) for visible unavailable choices or :if for hidden choices")
    );
    let choice = choice_statement(single_block(&lowered), 0);
    assert!(choice.availability_requirement.is_none());
}

#[test]
fn effect_arguments_preserve_scalar_types() {
    let source = concat!(
        ":: tavern_arrival\n",
        "! immediate debug_effect(\"door slam\", -3, 1.5, false, actor.id)\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let Statement::Effect(effect) = &single_block(&lowered).statements[0] else {
        panic!("expected effect");
    };
    assert_eq!(effect.mode, EffectMode::Immediate);
    assert_eq!(effect.function, "debug_effect");
    assert_eq!(
        effect.args,
        [
            ScalarValue::from("door slam").into(),
            ScalarValue::from(-3_i64).into(),
            ScalarValue::from(1.5_f64).into(),
            ScalarValue::from(false).into(),
            Argument::identifier("actor.id"),
        ]
    );
}

#[test]
fn diverts_parse_external_targets_and_extra_token_spans() {
    let source = concat!(
        ":: tavern_arrival\n",
        "-> local_news\n",
        "-> dialogue/market.recite::market_intro\n",
        "-> local_news extra\n",
        "-> dialogue/market.recite::\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(&lowered, ["RECITE_PARSE011", "RECITE_PARSE011"]);
    assert_eq!(lowered.diagnostics[0].span.start.line(), 4);
    assert_eq!(lowered.diagnostics[0].span.start.column(), 15);
    assert_eq!(lowered.diagnostics[1].span.start.line(), 5);
    assert_eq!(lowered.diagnostics[1].span.start.column(), 4);

    let block = single_block(&lowered);
    let Statement::Divert(local) = &block.statements[0] else {
        panic!("expected local divert");
    };
    assert_eq!(
        local.target,
        DivertTarget::Block(recite_core::BlockReference::local(
            recite_core::BlockId::new("local_news").expect("valid block id")
        ))
    );

    let Statement::Divert(external) = &block.statements[1] else {
        panic!("expected external divert");
    };
    assert_eq!(
        external.target,
        DivertTarget::Block(recite_core::BlockReference::external(
            "dialogue/market.recite",
            recite_core::BlockId::new("market_intro").expect("valid block id")
        ))
    );
}

#[test]
fn choice_extracts_first_divert_as_target_and_preserves_later_statement_order() {
    let source = concat!(
        ":: tavern_arrival\n",
        "? ask_road@c16635bfe6bc795f818b\n",
        "  Ask about the road.\n",
        "  -> road_intro\n",
        "  ! immediate play_sfx(page)\n",
        "  -> fallback_road\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let choice = choice_statement(single_block(&lowered), 0);
    assert_eq!(
        choice.target.as_ref().map(|target| &target.target),
        Some(&DivertTarget::Block(recite_core::BlockReference::local(
            recite_core::BlockId::new("road_intro").expect("valid block id")
        )))
    );
    assert_eq!(
        choice
            .target
            .as_ref()
            .map(|target| target.span.start.line()),
        Some(4)
    );
    assert_eq!(
        choice
            .statements
            .iter()
            .map(Statement::kind)
            .collect::<Vec<_>>(),
        [StatementKind::Effect, StatementKind::Divert]
    );
}
