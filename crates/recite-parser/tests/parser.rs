use recite_core::{ScalarValue, SpeakerId, Statement, Value};
use recite_parser::parse;

#[test]
fn syntax_tree_round_trips_source_text() {
    let source = concat!(
        ":: tavern_arrival default\r\n",
        "> ta_001 speaker=innkeeper\r\n",
        "  Welcome [slow]back[/slow].\r\n",
    );

    let parse = parse("dialogue/tavern.recite", source);

    assert_eq!(parse.syntax().text().to_string(), source);
    assert!(parse.diagnostics().is_empty());
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

    let lowered = parse("dialogue/tavern.recite", source).lower_source_file();

    assert!(lowered.diagnostics.is_empty());
    assert_eq!(lowered.source_file.path, "dialogue/tavern.recite");
    assert_eq!(lowered.source_file.blocks.len(), 1);

    let block = &lowered.source_file.blocks[0];
    assert_eq!(block.id.as_str(), "tavern_arrival");
    assert!(block.is_default);
    assert_eq!(block.statements.len(), 2);

    let Statement::Line(first_line) = &block.statements[0] else {
        panic!("expected first lowered statement to be a line");
    };
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

    let Statement::Line(second_line) = &block.statements[1] else {
        panic!("expected second lowered statement to be a line");
    };
    assert_eq!(
        second_line.id.as_ref().map(recite_core::LineId::as_str),
        Some("ta_002")
    );
    assert_eq!(second_line.source_text.text, "What do you need?");
}

#[test]
fn lowering_reports_unsupported_headers_without_losing_syntax() {
    let source = concat!(
        ":: tavern_arrival\n",
        "? ta_choice\n",
        "  Ask about the road.\n",
    );

    let parse = parse("dialogue/tavern.recite", source);
    let lowered = parse.lower_source_file();

    assert_eq!(parse.syntax().text().to_string(), source);
    assert_eq!(lowered.diagnostics.len(), 1);
    assert_eq!(lowered.diagnostics[0].code.as_str(), "RECITE_PARSE004");
    assert_eq!(lowered.source_file.blocks[0].statements.len(), 0);
}

#[test]
fn line_lowering_preserves_ordered_metadata_and_speaker() {
    let source = concat!(
        ":: tavern_arrival\n",
        "> ta_001 speaker=innkeeper portrait=neutral sfx=door sfx=mug repeat=true count=2\n",
        "  Welcome.\n",
    );

    let lowered = parse("dialogue/tavern.recite", source).lower_source_file();

    assert!(lowered.diagnostics.is_empty());
    let Statement::Line(line) = &lowered.source_file.blocks[0].statements[0] else {
        panic!("expected lowered line");
    };

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

    let lowered = parse("dialogue/tavern.recite", source).lower_source_file();

    assert_eq!(lowered.diagnostics.len(), 1);
    assert_eq!(lowered.diagnostics[0].code.as_str(), "RECITE_PARSE004");
    assert_eq!(lowered.source_file.blocks[0].statements.len(), 1);

    let Statement::Line(line) = &lowered.source_file.blocks[0].statements[0] else {
        panic!("expected ordinary sibling line");
    };
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

    let lowered = parse("dialogue/tavern.recite", source).lower_source_file();

    assert_eq!(lowered.diagnostics.len(), 1);
    assert_eq!(lowered.diagnostics[0].code.as_str(), "RECITE_PARSE004");

    let Statement::Line(line) = &lowered.source_file.blocks[0].statements[0] else {
        panic!("expected prompt line");
    };
    assert_eq!(
        line.source_text.text,
        "What do you need?\nStill parent prose."
    );
}
