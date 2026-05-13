use expect_test::{Expect, expect};
use recite_core::{
    Argument, Block, Choice, ChoiceEcho, ConditionExpression, DivertTarget, EffectMode, IfBranch,
    Line, MatchBranch, MatchPattern, ScalarValue, SpeakerId, Statement, StatementKind, Value,
};
use recite_parser::{LoweredSourceFile, ReciteSyntaxKind, parse};

const TEST_PATH: &str = "dialogue/tavern.recite";

#[test]
fn syntax_tree_round_trips_source_text() {
    let source = concat!(
        ":: tavern_arrival default\r\n",
        "> ta_001 speaker=innkeeper\r\n",
        "  Welcome [slow]back[/slow].\r\n",
    );

    let parse = parse(TEST_PATH, source);

    assert_eq!(parse.syntax().text().to_string(), source);
    assert!(parse.diagnostics().is_empty());
}

#[test]
fn statement_markers_classify_consistently() {
    let source = concat!(
        ":: tavern\n",
        "> line\n",
        "? choice\n",
        "! deferred effect\n",
        "-> END\n",
        ":if knows_secret(player)\n",
        ":else\n",
        ":match thread_stage(thread)\n",
        ":case _\n",
        "# comment\n",
        "  prose\n",
        "oops\n",
    );

    let parse = parse(TEST_PATH, source);
    let kinds = parse
        .syntax()
        .children()
        .map(|node| node.kind())
        .collect::<Vec<_>>();

    assert_eq!(
        kinds,
        [
            ReciteSyntaxKind::Block,
            ReciteSyntaxKind::Line,
            ReciteSyntaxKind::Choice,
            ReciteSyntaxKind::Effect,
            ReciteSyntaxKind::Divert,
            ReciteSyntaxKind::If,
            ReciteSyntaxKind::Else,
            ReciteSyntaxKind::Match,
            ReciteSyntaxKind::Case,
            ReciteSyntaxKind::Comment,
            ReciteSyntaxKind::Prose,
            ReciteSyntaxKind::Error,
        ]
    );
}

#[test]
fn directive_markers_are_boundary_aware() {
    let source = concat!(":ifx\n", ":elsewhere\n", ":matchmaking\n", ":casefile\n");

    let parse = parse(TEST_PATH, source);
    let kinds = parse
        .syntax()
        .children()
        .map(|node| node.kind())
        .collect::<Vec<_>>();

    assert_eq!(
        kinds,
        [
            ReciteSyntaxKind::Error,
            ReciteSyntaxKind::Error,
            ReciteSyntaxKind::Error,
            ReciteSyntaxKind::Error,
        ]
    );
    assert_eq!(
        parse
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        [
            "RECITE_PARSE001",
            "RECITE_PARSE001",
            "RECITE_PARSE001",
            "RECITE_PARSE001",
        ]
    );
}

#[test]
fn directive_like_prose_does_not_terminate_line_bodies() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> ta_001\n",
        "  :ifx this is prose, not a directive.\n",
        "  :casefile is also prose.\n",
        "  :matchmaking remains prose.\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    assert_eq!(
        line_statement(single_block(&lowered), 0).source_text.text,
        ":ifx this is prose, not a directive.\n:casefile is also prose.\n:matchmaking remains prose."
    );
}

#[test]
fn syntax_tree_recovers_malformed_lines_with_stable_diagnostics() {
    let parse = parse("dialogue/broken.recite", "oops\n:: tavern\n");

    assert_eq!(parse.syntax().text().to_string(), "oops\n:: tavern\n");
    assert_eq!(parse.diagnostics().len(), 1);
    assert_eq!(parse.diagnostics()[0].code.as_str(), "RECITE_PARSE001");
    assert_eq!(parse.diagnostics()[0].span.file, "dialogue/broken.recite");
    assert_eq!(parse.diagnostics()[0].span.start.line(), 1);
    assert_eq!(parse.diagnostics()[0].span.start.column(), 1);
}

#[test]
fn lowering_produces_source_file_shape_and_preserves_ordered_text() {
    let source = concat!(
        ":: tavern_arrival default\n",
        "> ta_001 speaker=innkeeper portrait=neutral\n",
        "  Welcome to the Rusty Flagon.\n",
        "\n",
        "  Haven't seen you in a while.\n",
        "> ta_002\n",
        "  What do you need?\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    assert_eq!(lowered.source_file.path, TEST_PATH);
    assert_eq!(lowered.source_file.blocks.len(), 1);

    let block = single_block(&lowered);
    assert_eq!(block.id.as_str(), "tavern_arrival");
    assert!(block.is_default);
    assert_eq!(block.statements.len(), 2);

    let first_line = line_statement(block, 0);
    assert_eq!(
        first_line.id.as_ref().map(recite_core::LineId::as_str),
        Some("ta_001")
    );
    assert_eq!(
        first_line.speaker.as_ref().map(SpeakerId::as_str),
        Some("innkeeper")
    );
    assert_eq!(
        first_line
            .metadata
            .iter()
            .map(|entry| (entry.key.as_str(), &entry.value))
            .collect::<Vec<_>>(),
        [(
            "portrait",
            &Value::Scalar(ScalarValue::String("neutral".to_owned()))
        )]
    );
    assert_eq!(
        first_line.source_text.text,
        "Welcome to the Rusty Flagon.\n\nHaven't seen you in a while."
    );
    assert_eq!(first_line.span.start.line(), 2);
    assert_eq!(first_line.source_text.span.start.line(), 3);

    let second_line = line_statement(block, 1);
    assert_eq!(
        second_line.id.as_ref().map(recite_core::LineId::as_str),
        Some("ta_002")
    );
    assert_eq!(second_line.source_text.text, "What do you need?");
}

#[test]
fn lowering_reports_mixed_indent_inside_line_body() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> ta_001\n",
        "  Welcome.\n",
        "    This line uses a different indent.\n",
        "  Back to the original indent.\n",
    );

    let parse = parse(TEST_PATH, source);
    let lowered = parse.lower_source_file();

    assert_eq!(
        parse
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        ["RECITE_PARSE007"]
    );
    assert_diagnostic_codes(&lowered, ["RECITE_PARSE007"]);
    assert_eq!(lowered.diagnostics[0].span.start.line(), 4);
    assert_eq!(lowered.diagnostics[0].span.start.column(), 5);

    let line = line_statement(single_block(&lowered), 0);
    assert_eq!(
        line.source_text.text,
        "Welcome.\nBack to the original indent."
    );
}

#[test]
fn mixed_indent_statement_markers_report_indent_diagnostics() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> ta_001\n",
        "  Welcome.\n",
        "    ? ask_road\n",
        "    :if knows_secret(player)\n",
        "  Back to the original indent.\n",
    );

    let parse = parse(TEST_PATH, source);
    let lowered = parse.lower_source_file();

    assert_eq!(
        parse
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        ["RECITE_PARSE007", "RECITE_PARSE007"]
    );
    assert_diagnostic_codes(&lowered, ["RECITE_PARSE007", "RECITE_PARSE007"]);
    assert_eq!(lowered.diagnostics[0].span.start.line(), 4);
    assert_eq!(lowered.diagnostics[0].span.start.column(), 5);
    assert_eq!(lowered.diagnostics[1].span.start.line(), 5);
    assert_eq!(lowered.diagnostics[1].span.start.column(), 5);

    let line = line_statement(single_block(&lowered), 0);
    assert_eq!(
        line.source_text.text,
        "Welcome.\nBack to the original indent."
    );
}

#[test]
fn sibling_indented_statement_headers_terminate_line_prose() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> before_choice\n",
        "  Choice prompt.\n",
        "  ? ask_road\n",
        "    Ask about the road.\n",
        "> before_effect\n",
        "  Effect prompt.\n",
        "  ! deferred play_sfx(door)\n",
        "> before_divert\n",
        "  Divert prompt.\n",
        "  -> END\n",
        "> before_line\n",
        "  Line prompt.\n",
        "  > nested_line\n",
        "    Nested text.\n",
        "> before_if\n",
        "  If prompt.\n",
        "  :if knows_secret(player)\n",
        "    > gated_line\n",
        "      Gated text.\n",
        "> before_block\n",
        "  Block prompt.\n",
        ":: next_block\n",
        "> next_line\n",
        "  Next block text.\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());

    let first_block = &lowered.source_file.blocks[0];
    assert_eq!(
        (0..5)
            .map(|index| line_statement(first_block, index).source_text.text.as_str())
            .collect::<Vec<_>>(),
        [
            "Choice prompt.",
            "Effect prompt.",
            "Divert prompt.",
            "Line prompt.",
            "If prompt.",
        ]
    );
    assert_eq!(
        line_statement(first_block, 5).source_text.text,
        "Block prompt."
    );

    assert_eq!(
        line_statement(first_block, 0)
            .statements
            .iter()
            .map(Statement::kind)
            .collect::<Vec<_>>(),
        [StatementKind::Choice]
    );
    assert_eq!(
        line_statement(first_block, 1)
            .statements
            .iter()
            .map(Statement::kind)
            .collect::<Vec<_>>(),
        [StatementKind::Effect]
    );
    assert_eq!(
        line_statement(first_block, 2)
            .statements
            .iter()
            .map(Statement::kind)
            .collect::<Vec<_>>(),
        [StatementKind::Divert]
    );
    assert_eq!(
        line_statement(first_block, 3)
            .statements
            .iter()
            .map(Statement::kind)
            .collect::<Vec<_>>(),
        [StatementKind::Line]
    );
    assert_eq!(
        line_statement(first_block, 4)
            .statements
            .iter()
            .map(Statement::kind)
            .collect::<Vec<_>>(),
        [StatementKind::If]
    );
    assert_eq!(lowered.source_file.blocks[1].id.as_str(), "next_block");
}

#[test]
fn multiple_nested_statements_do_not_promote_to_block_statements() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> prompt_line\n",
        "  What do you need?\n",
        "  ? ask_road\n",
        "    Ask about the road.\n",
        "  > nested_line\n",
        "    Nested line text.\n",
        "> after_prompt\n",
        "  Carry on.\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let block = single_block(&lowered);
    assert_eq!(block.statements.len(), 2);

    let prompt = line_statement(block, 0);
    assert_eq!(
        prompt.id.as_ref().map(recite_core::LineId::as_str),
        Some("prompt_line")
    );
    assert_eq!(prompt.source_text.text, "What do you need?");
    assert_eq!(
        prompt
            .statements
            .iter()
            .map(Statement::kind)
            .collect::<Vec<_>>(),
        [StatementKind::Choice, StatementKind::Line]
    );

    let after = line_statement(block, 1);
    assert_eq!(
        after.id.as_ref().map(recite_core::LineId::as_str),
        Some("after_prompt")
    );
    assert_eq!(after.source_text.text, "Carry on.");
}

#[test]
fn lowering_parses_top_level_choices_without_losing_syntax() {
    let source = concat!(
        ":: tavern_arrival\n",
        "? ta_choice\n",
        "  Ask about the road.\n",
    );

    let parse = parse(TEST_PATH, source);
    let lowered = parse.lower_source_file();

    assert_eq!(parse.syntax().text().to_string(), source);
    assert!(lowered.diagnostics.is_empty());

    let choice = choice_statement(single_block(&lowered), 0);
    assert_eq!(
        choice.id.as_ref().map(recite_core::ChoiceId::as_str),
        Some("ta_choice")
    );
    assert_eq!(choice.source_text.text, "Ask about the road.");
}

#[test]
fn lowering_preserves_block_comments_in_source_order() {
    let source = concat!(
        ":: tavern_arrival\n",
        "# scene opener\n",
        "> ta_001\n",
        "  Welcome.\n",
        "# outro marker\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let statements = &single_block(&lowered).statements;
    assert_eq!(
        statements.iter().map(Statement::kind).collect::<Vec<_>>(),
        [
            StatementKind::Comment,
            StatementKind::Line,
            StatementKind::Comment
        ]
    );

    let first_comment = comment_statement(single_block(&lowered), 0);
    assert_eq!(first_comment.text, "scene opener");
    assert_eq!(first_comment.span.start.line(), 2);
    assert_eq!(first_comment.span.start.column(), 1);

    let line = line_statement(single_block(&lowered), 1);
    assert_eq!(line.source_text.text, "Welcome.");

    let second_comment = comment_statement(single_block(&lowered), 2);
    assert_eq!(second_comment.text, "outro marker");
    assert_eq!(second_comment.span.start.line(), 5);
}

#[test]
fn line_lowering_preserves_ordered_metadata_and_speaker() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> ta_001 speaker=innkeeper portrait=neutral sfx=door sfx=mug repeat=true count=2\n",
        "  Welcome.\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let line = line_statement(single_block(&lowered), 0);

    assert_eq!(
        line.speaker.as_ref().map(SpeakerId::as_str),
        Some("innkeeper")
    );
    assert_eq!(
        line.metadata
            .iter()
            .map(|entry| (entry.key.as_str(), &entry.value))
            .collect::<Vec<_>>(),
        [
            (
                "portrait",
                &Value::Scalar(ScalarValue::String("neutral".to_owned()))
            ),
            (
                "sfx",
                &Value::Scalar(ScalarValue::String("door".to_owned()))
            ),
            ("sfx", &Value::Scalar(ScalarValue::String("mug".to_owned()))),
            ("repeat", &Value::Scalar(ScalarValue::Boolean(true))),
            ("count", &Value::Scalar(ScalarValue::Integer(2))),
        ]
    );

    let first_metadata = &line.metadata.as_slice()[0];
    assert_eq!(
        first_metadata.source_span.as_ref().unwrap().start.column(),
        28
    );
    assert_eq!(
        first_metadata
            .source_span
            .as_ref()
            .unwrap()
            .end
            .unwrap()
            .column(),
        43
    );
    assert_eq!(first_metadata.key_span.as_ref().unwrap().start.column(), 28);
    assert_eq!(
        first_metadata.value_span.as_ref().unwrap().start.column(),
        37
    );
}

#[test]
fn malformed_block_header_fields_are_reported() {
    let source = concat!(
        ":: id=bad\n",
        ":: tavern_arrival default bare speaker=\n",
        "> ta_001\n",
        "  Hello.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(
        &lowered,
        ["RECITE_PARSE003", "RECITE_PARSE008", "RECITE_PARSE008"],
    );
    let block = single_block(&lowered);
    assert_eq!(block.id.as_str(), "tavern_arrival");
    assert!(block.is_default);
    assert!(block.default_speaker.is_none());
}

#[test]
fn lowering_parses_statement_vocabulary_and_conditions() {
    let source = concat!(
        ":: tavern_arrival default speaker=innkeeper scene=opening scene=repeat\n",
        "# scene opener\n",
        "> prompt speaker=innkeeper portrait=neutral sfx=door sfx=mug\n",
        "  What do you need?\n",
        "\n",
        "  ? ask_news echo=selected_text sfx=paper if familiarity_gte(hazel, rhea, 3)\n",
        "    What's the news?\n",
        "    -> local_news\n",
        ":if not thread_completed(rhea_job_response) and familiarity_gte(hazel, rhea, 3)\n",
        "  > gated_line\n",
        "    Still waiting.\n",
        ":else\n",
        "  > fallback_line\n",
        "    Fine.\n",
        "! deferred advance_thread(rhea_job_response, tired)\n",
        ":match thread_stage(rhea_job_response)\n",
        "  :case tired\n",
        "    > tired_line\n",
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
        Some("ask_news")
    );
    assert_eq!(choice.echo, ChoiceEcho::SelectedText);
    assert_eq!(choice.source_text.text, "What's the news?");
    assert!(choice.condition.is_some());
    assert_eq!(
        choice.target,
        Some(DivertTarget::Block(recite_core::BlockReference::local(
            recite_core::BlockId::new("local_news").expect("valid block id")
        )))
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
fn condition_precedence_and_grouping_are_lowered() {
    let source = concat!(
        ":: tavern_arrival\n",
        ":if knows_a() or knows_b() and not (blocked())\n",
        "  > gated_line\n",
        "    Hi.\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let branch = if_statement(single_block(&lowered), 0);
    let ConditionExpression::Or(or_group) = &branch.condition else {
        panic!("expected top-level or condition");
    };
    assert_eq!(or_group.expressions.len(), 2);
    let ConditionExpression::And(and_group) = &or_group.expressions[1] else {
        panic!("expected and to bind tighter than or");
    };
    assert_eq!(and_group.expressions.len(), 2);
    let ConditionExpression::Not(not) = &and_group.expressions[1] else {
        panic!("expected not expression");
    };
    assert!(matches!(
        not.expression.as_ref(),
        ConditionExpression::Grouped(_)
    ));
}

#[test]
fn condition_parser_rejects_dangling_and_trailing_tokens() {
    let source = concat!(
        ":: tavern_arrival\n",
        ":if knows_secret(player) trailing\n",
        "  > trailing_tokens\n",
        "    Hi.\n",
        ":if knows_a() and\n",
        "  > dangling_operator\n",
        "    Hi.\n",
        "? ask if knows_secret(player) sfx=door\n",
        "  Ask.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(
        &lowered,
        ["RECITE_PARSE013", "RECITE_PARSE013", "RECITE_PARSE013"],
    );

    let choice = choice_statement(single_block(&lowered), 0);
    assert_eq!(
        choice.id.as_ref().map(recite_core::ChoiceId::as_str),
        Some("ask")
    );
    assert!(choice.condition.is_none());
    assert!(choice.metadata.is_empty());
}

#[test]
fn effect_parser_rejects_incomplete_or_trailing_calls() {
    let source = concat!(
        ":: tavern_arrival\n",
        "! deferred play_sfx(snap) trailing\n",
        "! immediate play_sfx(\n",
        "! blocking\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(
        &lowered,
        ["RECITE_PARSE012", "RECITE_PARSE012", "RECITE_PARSE012"],
    );
    assert!(single_block(&lowered).statements.is_empty());
}

#[test]
fn else_only_attaches_to_immediately_preceding_if() {
    let source = concat!(
        ":: tavern_arrival\n",
        ":if knows_secret(player)\n",
        "  > gated_line\n",
        "    Hi.\n",
        "# comment breaks the if/else adjacency\n",
        ":else\n",
        "  > fallback_line\n",
        "    Hi.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(&lowered, ["RECITE_PARSE015"]);
    let block = single_block(&lowered);
    assert_eq!(
        block
            .statements
            .iter()
            .map(Statement::kind)
            .collect::<Vec<_>>(),
        [StatementKind::If, StatementKind::Comment]
    );
    assert!(if_statement(block, 0).else_statements.is_empty());
}

#[test]
fn malformed_headers_conditions_and_cases_report_diagnostics() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> line bare key=\n",
        "  Hello.\n",
        "? if knows_secret(player)\n",
        "  Choice.\n",
        "->\n",
        "! delayed play()\n",
        ":if knows_secret(\n",
        "  > gated\n",
        "    Hi.\n",
        ":else trailing\n",
        "  > fallback\n",
        "    Hi.\n",
        ":case tired\n",
        "  > orphan\n",
        "    No.\n",
        ":match thread_stage(thread)\n",
        "  > not_case\n",
        "    No.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(
        &lowered,
        [
            "RECITE_PARSE008",
            "RECITE_PARSE008",
            "RECITE_PARSE009",
            "RECITE_PARSE010",
            "RECITE_PARSE012",
            "RECITE_PARSE013",
            "RECITE_PARSE008",
            "RECITE_PARSE016",
            "RECITE_PARSE014",
        ],
    );
}

#[test]
fn conditional_body_is_not_flattened_into_block_statements() {
    let source = concat!(
        ":: tavern_arrival\n",
        ":if knows_secret(player)\n",
        "  > gated_line\n",
        "    You know the password.\n",
        "> ordinary_line\n",
        "  Welcome.\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    assert_eq!(lowered.source_file.blocks[0].statements.len(), 2);

    let branch = if_statement(single_block(&lowered), 0);
    assert_eq!(branch.then_statements.len(), 1);
    let Statement::Line(gated_line) = &branch.then_statements[0] else {
        panic!("expected gated line");
    };
    assert_eq!(
        gated_line.id.as_ref().map(recite_core::LineId::as_str),
        Some("gated_line")
    );

    let line = line_statement(single_block(&lowered), 1);
    assert_eq!(
        line.id.as_ref().map(recite_core::LineId::as_str),
        Some("ordinary_line")
    );
    assert_eq!(line.source_text.text, "Welcome.");
}

#[test]
fn nested_choice_body_is_not_appended_to_parent_prose() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> prompt_line\n",
        "  What do you need?\n",
        "  ? ask_road\n",
        "    Ask about the road.\n",
        "  Still parent prose.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(&lowered, ["RECITE_PARSE017"]);

    let line = line_statement(single_block(&lowered), 0);
    assert_eq!(line.source_text.text, "What do you need?");
    assert_eq!(
        line.statements
            .iter()
            .map(Statement::kind)
            .collect::<Vec<_>>(),
        [StatementKind::Choice]
    );
}

#[test]
fn lowering_summary_stays_stable_for_supported_and_recovered_statements() {
    let source = concat!(
        ":: tavern_arrival default\n",
        "# scene opener\n",
        "> ta_001 speaker=innkeeper portrait=neutral repeat=true\n",
        "  Welcome.\n",
        "  ? ask_road\n",
        "    Ask about the road.\n",
        "  Still parent prose.\n",
        "? unsupported_choice\n",
        "  Ask about work.\n",
    );

    let lowered = lower(source);

    assert_snapshot(
        &lowered_summary(&lowered),
        expect![[r#"
            diagnostics:
              - RECITE_PARSE017 @ 7:3
            blocks:
              - tavern_arrival default=true statements=3
                - comment "scene opener" @ 2:1
                - line ta_001 speaker=innkeeper text="Welcome." metadata=[portrait, repeat]
                - Choice
        "#]],
    );
}

fn lower(source: &str) -> LoweredSourceFile {
    parse(TEST_PATH, source).lower_source_file()
}

fn single_block(lowered: &LoweredSourceFile) -> &Block {
    assert_eq!(lowered.source_file.blocks.len(), 1);
    &lowered.source_file.blocks[0]
}

fn line_statement(block: &Block, index: usize) -> &Line {
    let Statement::Line(line) = &block.statements[index] else {
        panic!("expected statement {index} to be a line");
    };

    line
}

fn choice_statement(block: &Block, index: usize) -> &Choice {
    let Statement::Choice(choice) = &block.statements[index] else {
        panic!("expected statement {index} to be a choice");
    };

    choice
}

fn nested_choice(line: &Line, index: usize) -> &Choice {
    let Statement::Choice(choice) = &line.statements[index] else {
        panic!("expected nested statement {index} to be a choice");
    };

    choice
}

fn if_statement(block: &Block, index: usize) -> &IfBranch {
    let Statement::If(branch) = &block.statements[index] else {
        panic!("expected statement {index} to be an if branch");
    };

    branch
}

fn match_statement(block: &Block, index: usize) -> &MatchBranch {
    let Statement::Match(branch) = &block.statements[index] else {
        panic!("expected statement {index} to be a match branch");
    };

    branch
}

fn comment_statement(block: &Block, index: usize) -> &recite_core::Comment {
    let Statement::Comment(comment) = &block.statements[index] else {
        panic!("expected statement {index} to be a comment");
    };

    comment
}

fn assert_diagnostic_codes<const N: usize>(lowered: &LoweredSourceFile, expected: [&str; N]) {
    assert_eq!(
        lowered
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        expected
    );
}

fn assert_snapshot(actual: &str, expected: Expect) {
    expected.assert_eq(actual);
}

fn lowered_summary(lowered: &LoweredSourceFile) -> String {
    let mut summary = String::new();

    summary.push_str("diagnostics:\n");
    if lowered.diagnostics.is_empty() {
        summary.push_str("  <none>\n");
    } else {
        for diagnostic in &lowered.diagnostics {
            summary.push_str(&format!(
                "  - {} @ {}:{}\n",
                diagnostic.code.as_str(),
                diagnostic.span.start.line(),
                diagnostic.span.start.column()
            ));
        }
    }

    summary.push_str("blocks:\n");
    for block in &lowered.source_file.blocks {
        summary.push_str(&format!(
            "  - {} default={} statements={}\n",
            block.id.as_str(),
            block.is_default,
            block.statements.len()
        ));

        for statement in &block.statements {
            match statement {
                Statement::Comment(comment) => summary.push_str(&format!(
                    "    - comment {:?} @ {}:{}\n",
                    comment.text,
                    comment.span.start.line(),
                    comment.span.start.column()
                )),
                Statement::Line(line) => summary.push_str(&format!(
                    "    - line {} speaker={} text={:?} metadata=[{}]\n",
                    line.id
                        .as_ref()
                        .map(recite_core::LineId::as_str)
                        .unwrap_or("<missing>"),
                    line.speaker
                        .as_ref()
                        .map(SpeakerId::as_str)
                        .unwrap_or("<none>"),
                    line.source_text.text,
                    line.metadata
                        .iter()
                        .map(|entry| entry.key.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )),
                other => summary.push_str(&format!("    - {:?}\n", other.kind())),
            }
        }
    }

    summary
}
