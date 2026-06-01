#![cfg(test)]

use recite_core::*;

#[test]
fn source_ast_represents_dialogue_constructs_with_spans() {
    let source_file = representative_source_file();
    let block = &source_file.blocks[0];

    assert_eq!(source_file.path, "dialogue/tavern.recite");
    assert_eq!(block.id.as_str(), "tavern_arrival");
    assert!(block.is_default);
    assert_eq!(
        block.default_speaker.as_ref().map(SpeakerId::as_str),
        Some("innkeeper")
    );

    let Statement::Line(line) = &block.statements[1] else {
        panic!("expected prompt line");
    };
    assert_eq!(line.id.as_ref().map(LineId::as_str), Some("ta_prompt_001"));
    assert_eq!(line.source_text.text, "What do you need?");
    assert_eq!(line.source_text.span.start.line(), 4);
    assert_eq!(
        line.metadata
            .iter()
            .map(|entry| entry.key.as_str())
            .collect::<Vec<_>>(),
        ["sfx", "portrait", "sfx"]
    );

    let Statement::Choice(choice) = &line.statements[0] else {
        panic!("expected nested choice");
    };
    assert_eq!(
        choice.id.as_ref().map(ChoiceId::as_str),
        Some("ta_opt_news")
    );
    assert!(choice.condition.is_some());
    assert_eq!(
        choice.target.as_ref().map(|target| &target.target),
        Some(&DivertTarget::End)
    );
    assert_eq!(choice.echo, ChoiceEcho::None);

    let Statement::Effect(effect) = &block.statements[4] else {
        panic!("expected effect");
    };
    assert_eq!(effect.mode, EffectMode::Blocking);
    assert_eq!(effect.function, "mark_map");
}

#[test]
fn condition_expressions_preserve_composite_spans() {
    let completed = ConditionExpression::call(
        "thread_completed",
        vec![Argument::identifier("rhea_job_response")],
        span(3, 9),
    );
    let grouped = ConditionExpression::grouped(completed, span(3, 8));
    let not = ConditionExpression::not(grouped, span(3, 4));
    let known = ConditionExpression::call(
        "familiarity_gte",
        vec![
            Argument::identifier("hazel"),
            Argument::identifier("rhea"),
            ScalarValue::from(3_i64).into(),
        ],
        span(3, 40),
    );
    let and = ConditionExpression::and(vec![not, known], span(3, 4));
    let fallback = ConditionExpression::call("debug_override", Vec::new(), span(4, 4));
    let or = ConditionExpression::or(vec![and, fallback], span(3, 4));

    assert_eq!(or.span(), &span(3, 4));
    let ConditionExpression::Or(or_group) = &or else {
        panic!("expected or condition");
    };
    assert_eq!(or_group.span, span(3, 4));

    let ConditionExpression::And(and_group) = &or_group.expressions[0] else {
        panic!("expected and condition");
    };
    assert_eq!(and_group.span, span(3, 4));

    let ConditionExpression::Not(not_unary) = &and_group.expressions[0] else {
        panic!("expected not condition");
    };
    assert_eq!(not_unary.span, span(3, 4));

    let ConditionExpression::Grouped(grouped_unary) = not_unary.expression.as_ref() else {
        panic!("expected grouped condition");
    };
    assert_eq!(grouped_unary.span, span(3, 8));
}

#[test]
fn depth_first_traversal_preserves_source_order() {
    let source_file = representative_source_file();
    let mut kinds = Vec::new();
    source_file.visit_statements_depth_first(&mut |statement| kinds.push(statement.kind()));

    assert_eq!(
        kinds,
        [
            StatementKind::Comment,
            StatementKind::Line,
            StatementKind::Choice,
            StatementKind::Divert,
            StatementKind::If,
            StatementKind::Line,
            StatementKind::Line,
            StatementKind::Effect,
            StatementKind::Effect,
            StatementKind::Match,
            StatementKind::Line,
            StatementKind::Effect,
            StatementKind::Line,
        ]
    );
}

#[test]
fn missing_line_and_choice_ids_are_representable_before_validation() {
    let line = Line::new(None, SourceText::new("Hello.", span(2, 3)), span(1, 1));
    let choice = Choice::new(None, SourceText::new("Leave.", span(5, 3)), span(4, 1));

    assert_eq!(line.id, None);
    assert_eq!(choice.id, None);
}

fn representative_source_file() -> SourceFile {
    let mut metadata = SourceMetadata::new();
    metadata.push(SourceMetadataEntry::new(
        "sfx",
        SourceMetadataScalar::Symbol("door".to_owned()),
    ));
    metadata.push(SourceMetadataEntry::new(
        "portrait",
        SourceMetadataScalar::Symbol("neutral".to_owned()),
    ));
    metadata.push(
        SourceMetadataEntry::new("sfx", SourceMetadataScalar::Symbol("mug".to_owned()))
            .with_source_span(span(3, 20)),
    );

    let choice = Choice::new(
        Some(ChoiceId::new("ta_opt_news").expect("valid choice ID")),
        SourceText::new("What's the news?", span(8, 5)),
        span(7, 3),
    )
    .with_condition(ConditionExpression::call(
        "familiarity_gte",
        vec![
            Argument::identifier("hazel"),
            Argument::identifier("innkeeper"),
            ScalarValue::from(3_i64).into(),
        ],
        span(7, 19),
    ))
    .with_target(ChoiceTarget::new(DivertTarget::End, span(9, 5)))
    .with_statements(vec![Statement::Divert(Divert::new(
        DivertTarget::Block(BlockReference::local(
            BlockId::new("local_news").expect("valid block ID"),
        )),
        span(9, 5),
    ))]);

    let line = Line::new(
        Some(LineId::new("ta_prompt_001").expect("valid line ID")),
        SourceText::new("What do you need?", span(4, 3)),
        span(3, 1),
    )
    .with_speaker(SpeakerId::new("innkeeper").expect("valid speaker ID"))
    .with_metadata(metadata)
    .with_statements(vec![Statement::Choice(choice)]);

    let branch = IfBranch::new(
        ConditionExpression::not(
            ConditionExpression::call(
                "thread_completed",
                vec![Argument::identifier("rhea_job_response")],
                span(11, 5),
            ),
            span(11, 1),
        ),
        vec![Statement::Line(Line::new(
            Some(LineId::new("thread_open_001").expect("valid line ID")),
            SourceText::new("Still waiting on that answer.", span(12, 5)),
            span(11, 3),
        ))],
        span(11, 1),
    )
    .with_else_statements(vec![Statement::Line(Line::new(
        Some(LineId::new("thread_closed_001").expect("valid line ID")),
        SourceText::new("That settled it.", span(14, 5)),
        span(13, 3),
    ))]);

    let match_branch = MatchBranch::new(
        ConditionCall::new(
            "thread_stage",
            vec![Argument::identifier("rhea_job_response")],
            span(18, 8),
        ),
        vec![
            MatchArm::new(
                MatchPattern::Variant("tired".to_owned()),
                vec![
                    Statement::Line(Line::new(
                        Some(LineId::new("rhea_tired_001").expect("valid line ID")),
                        SourceText::new("I'm exhausted.", span(20, 7)),
                        span(19, 5),
                    )),
                    Statement::Effect(Effect::new(
                        EffectMode::Deferred,
                        "advance_thread",
                        vec![
                            Argument::identifier("rhea_job_response"),
                            Argument::identifier("tired"),
                        ],
                        span(21, 7),
                    )),
                ],
                span(19, 3),
            ),
            MatchArm::new(
                MatchPattern::Wildcard,
                vec![Statement::Line(Line::new(
                    Some(LineId::new("rhea_default_001").expect("valid line ID")),
                    SourceText::new("Hey.", span(24, 7)),
                    span(23, 5),
                ))],
                span(23, 3),
            ),
        ],
        span(18, 1),
    );

    SourceFile::new(
        "dialogue/tavern.recite",
        vec![
            Block::new(
                BlockId::new("tavern_arrival").expect("valid block ID"),
                vec![
                    Statement::Comment(Comment::new("scene opener", span(1, 1))),
                    Statement::Line(line),
                    Statement::If(branch),
                    Statement::Effect(Effect::new(
                        EffectMode::Immediate,
                        "play_sfx",
                        vec![Argument::identifier("snap")],
                        span(16, 1),
                    )),
                    Statement::Effect(Effect::new(
                        EffectMode::Blocking,
                        "mark_map",
                        vec![Argument::identifier("old_watchtower")],
                        span(17, 1),
                    )),
                    Statement::Match(match_branch),
                ],
                span(2, 1),
            )
            .with_default(true)
            .with_default_speaker(SpeakerId::new("innkeeper").expect("valid speaker ID")),
        ],
    )
}

fn span(line: u32, column: u32) -> SourceSpan {
    SourceSpan::point(
        "dialogue/tavern.recite",
        SourcePosition::new(line, column).expect("valid source position"),
    )
}
