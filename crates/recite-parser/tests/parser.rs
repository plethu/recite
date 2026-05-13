use expect_test::{Expect, expect};
use recite_core::{Block, Line, ScalarValue, SpeakerId, Statement, StatementKind, Value};
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
        "> before_else\n",
        "  Else prompt.\n",
        "  :else\n",
        "    > fallback_line\n",
        "      Fallback text.\n",
        "> before_block\n",
        "  Block prompt.\n",
        ":: next_block\n",
        "> next_line\n",
        "  Next block text.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(
        &lowered,
        [
            "RECITE_PARSE004",
            "RECITE_PARSE004",
            "RECITE_PARSE004",
            "RECITE_PARSE004",
            "RECITE_PARSE004",
            "RECITE_PARSE004",
        ],
    );

    let first_block = &lowered.source_file.blocks[0];
    assert_eq!(
        (0..6)
            .map(|index| line_statement(first_block, index).source_text.text.as_str())
            .collect::<Vec<_>>(),
        [
            "Choice prompt.",
            "Effect prompt.",
            "Divert prompt.",
            "Line prompt.",
            "If prompt.",
            "Else prompt.",
        ]
    );
    assert_eq!(
        line_statement(first_block, 6).source_text.text,
        "Block prompt."
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

    assert_diagnostic_codes(&lowered, ["RECITE_PARSE004"]);
    let block = single_block(&lowered);
    assert_eq!(block.statements.len(), 2);

    let prompt = line_statement(block, 0);
    assert_eq!(
        prompt.id.as_ref().map(recite_core::LineId::as_str),
        Some("prompt_line")
    );
    assert_eq!(prompt.source_text.text, "What do you need?");

    let after = line_statement(block, 1);
    assert_eq!(
        after.id.as_ref().map(recite_core::LineId::as_str),
        Some("after_prompt")
    );
    assert_eq!(after.source_text.text, "Carry on.");
}

#[test]
fn lowering_reports_unsupported_headers_without_losing_syntax() {
    let source = concat!(
        ":: tavern_arrival\n",
        "? ta_choice\n",
        "  Ask about the road.\n",
    );

    let parse = parse(TEST_PATH, source);
    let lowered = parse.lower_source_file();

    assert_eq!(parse.syntax().text().to_string(), source);
    assert_diagnostic_codes(&lowered, ["RECITE_PARSE004"]);
    assert_eq!(lowered.source_file.blocks[0].statements.len(), 0);
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
}

#[test]
fn unsupported_conditional_body_is_not_flattened_into_block_statements() {
    let source = concat!(
        ":: tavern_arrival\n",
        ":if knows_secret(player)\n",
        "  > gated_line\n",
        "    You know the password.\n",
        "> ordinary_line\n",
        "  Welcome.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(&lowered, ["RECITE_PARSE004"]);
    assert_eq!(lowered.source_file.blocks[0].statements.len(), 1);

    let line = line_statement(single_block(&lowered), 0);
    assert_eq!(
        line.id.as_ref().map(recite_core::LineId::as_str),
        Some("ordinary_line")
    );
    assert_eq!(line.source_text.text, "Welcome.");
}

#[test]
fn unsupported_nested_choice_body_is_not_appended_to_parent_prose() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> prompt_line\n",
        "  What do you need?\n",
        "  ? ask_road\n",
        "    Ask about the road.\n",
        "  Still parent prose.\n",
    );

    let lowered = lower(source);

    assert_diagnostic_codes(&lowered, ["RECITE_PARSE004"]);

    let line = line_statement(single_block(&lowered), 0);
    assert_eq!(line.source_text.text, "What do you need?");
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
              - RECITE_PARSE004 @ 5:3
              - RECITE_PARSE004 @ 8:1
            blocks:
              - tavern_arrival default=true statements=2
                - comment "scene opener" @ 2:1
                - line ta_001 speaker=innkeeper text="Welcome." metadata=[portrait, repeat]
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
