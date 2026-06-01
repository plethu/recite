use super::*;

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
            &SourceMetadataValue::Scalar(SourceMetadataScalar::Symbol("neutral".to_owned(),))
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
fn lowering_preserves_inline_markup_as_source_text() {
    let source = concat!(
        ":: tavern_arrival default\n",
        "> marked_line\n",
        "  [slow]Welcome[/slow]\n",
        "  [shake]Stay alert.[/shake]\n",
        "  ? ask_road\n",
        "    [slow]Ask about the road.[/slow]\n",
        "    -> END\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let line = line_statement(single_block(&lowered), 0);
    assert_eq!(
        line.source_text.text,
        "[slow]Welcome[/slow]\n[shake]Stay alert.[/shake]"
    );
    assert_eq!(
        nested_choice(line, 0).source_text.text,
        "[slow]Ask about the road.[/slow]"
    );
}

#[test]
fn lowering_leaves_malformed_markup_text_for_validation() {
    let source = concat!(
        ":: tavern_arrival default\n",
        "> marked_line\n",
        "  [slow]Welcome.\n",
    );

    let lowered = lower(source);

    assert!(lowered.diagnostics.is_empty());
    let line = line_statement(single_block(&lowered), 0);
    assert_eq!(line.source_text.text, "[slow]Welcome.");
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
fn mixed_indent_inside_nested_statement_bodies_reports_spans() {
    let source = concat!(
        ":: tavern_arrival\n",
        "? ask_road\n",
        "  Ask about the road.\n",
        "    Wrong choice indent.\n",
        ":if knows_secret(player)\n",
        "  ! immediate play_sfx(ok)\n",
        "    ! immediate wrong_if_indent()\n",
        ":match thread_stage(thread)\n",
        "    :case ready\n",
        "      ! immediate play_sfx(ok)\n",
        "  :case tired\n",
        ":match mood(player)\n",
        "  :case calm\n",
        "    ! immediate play_sfx(ok)\n",
        "      ! immediate wrong_case_indent()\n",
    );

    let parse = parse(TEST_PATH, source);
    let lowered = parse.lower_source_file();

    assert!(parse.diagnostics().is_empty());
    assert_diagnostic_codes(
        &lowered,
        [
            "RECITE_PARSE007",
            "RECITE_PARSE007",
            "RECITE_PARSE007",
            "RECITE_PARSE007",
        ],
    );
    assert_eq!(
        lowered
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.span.start.line(), diagnostic.span.start.column()))
            .collect::<Vec<_>>(),
        [(4, 5), (7, 5), (11, 3), (15, 7)]
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
