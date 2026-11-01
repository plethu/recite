use super::*;

#[test]
fn lowering_parses_statement_vocabulary_and_conditions() {
    let source = concat!(
        ":: tavern_arrival default speaker=innkeeper scene=opening scene=repeat\n",
        "# scene opener\n",
        "> speaker=innkeeper portrait=neutral sfx=door sfx=mug @mqbmky6\n",
        "  What do you need?\n",
        "\n",
        "  ? echo=selected_text sfx=paper requires=(familiarity_gte(hazel, rhea, 3)) @txkzs21\n",
        "    What's the news?\n",
        "    -> local_news\n",
        ":if not thread_completed(rhea_job_response) and familiarity_gte(hazel, rhea, 3)\n",
        "  > @gated_line\n",
        "    Still waiting.\n",
        ":else\n",
        "  > @fallback_line\n",
        "    Fine.\n",
        "! deferred advance_thread(rhea_job_response, tired)\n",
        ":match thread_stage(rhea_job_response)\n",
        "  :case tired\n",
        "    > @tired_line\n",
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
        Some("txkzs21")
    );
    assert_eq!(choice.echo, ChoiceEcho::SelectedText);
    assert_eq!(choice.source_text.text, "What's the news?");
    assert!(choice.availability_requirement.is_some());
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
        "? sfx=paper requires=(trust_gte(hazel, rhea, 3)) topic=rumours reason=innkeeper_trust_hint @va0w2vn\n",
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
        "? reason=innkeeper_trust_hint @evjpn4g\n",
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
        "? requires=(trust_gte(hazel, rhea, 3)) reason=trust_too_low(hazel) @kq9evvh\n",
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
        Some((2, 60))
    );
}

#[test]
fn malformed_choice_availability_clauses_report_diagnostics() {
    let source = concat!(
        ":: tavern_arrival\n",
        "? requires=(trust_gte( @d5ssws1\n",
        "  What's the news?\n",
        "? reason=trust_too_low( @ask_more\n",
        "  More?\n",
        "? requires=(trust_gte(hazel, rhea, 3)) requires=(trust_gte(hazel, rhea, 4)) reason=innkeeper_trust_hint reason=other @ask_again\n",
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
        "? if trust_gte(hazel, rhea, 3) @hcpcvvy\n",
        "  What's the news?\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(&lowered, ["RECITE_PARSE018"]);
    assert_eq!(
        lowered.diagnostics[0]
            .help_presentation
            .as_ref()
            .map(|presentation| presentation.id().as_str()),
        Some("diagnostic-parse-018-remediation-001")
    );
    assert!(lowered.diagnostics[0].help.is_none());
    assert!(lowered.diagnostics[0].record().is_ok());
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
fn choice_extracts_first_divert_as_target_and_preserves_later_statement_order() {
    let source = concat!(
        ":: tavern_arrival\n",
        "? @ask_road\n",
        "  Ask about the road.\n",
        "  -> road_intro\n",
        "  ! immediate play_sfx(page)\n",
        "  -> fallback_road\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let choice = choice_statement(single_block(&lowered), 0);
    let Some(ChoiceTarget {
        target: DivertTarget::Block(reference),
        ..
    }) = choice.target.as_ref()
    else {
        panic!("expected local choice target");
    };
    assert_eq!(reference.block_id.as_str(), "road_intro");
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
