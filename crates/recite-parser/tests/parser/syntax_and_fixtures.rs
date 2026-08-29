use super::*;

#[test]
fn syntax_tree_round_trips_source_text() {
    let source = concat!(
        ":: tavern_arrival default\r\n",
        "> ta_001@adfa366c452c0c649fb2 speaker=innkeeper\r\n",
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
        "> line@1ae75ebb2fae238d8ade\n",
        "? choice@e0541a57a16607b99cb5\n",
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
        "> ta_001@0bf7b16fcf5fcbc4a0aa\n",
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
fn valid_fixture_has_no_parser_or_lowering_diagnostics() {
    const FIXTURE: &str = "fixtures/recite/valid/core_language_spike.recite";

    let source = fixture_source(FIXTURE);
    let parse = parse(FIXTURE, source.as_str());
    let lowered = parse.lower_source_file();

    assert!(parse.diagnostics().is_empty());
    assert!(lowered.diagnostics.is_empty());
    assert_diagnostic_snapshot(&lowered.diagnostics, diagnostic_snapshot_name(FIXTURE));
}

#[test]
fn valid_fixture_snapshots_lowered_source_shape() {
    const FIXTURE: &str = "fixtures/recite/valid/core_language_spike.recite";

    let source = fixture_source(FIXTURE);
    let lowered = parse(FIXTURE, source.as_str()).lower_source_file();

    fixture_support::assert_text_snapshot(
        &lowered_fixture_summary(&lowered),
        lowered_snapshot_name(FIXTURE),
    );
}

#[test]
fn shared_language_pressure_fixture_round_trips_and_lowers_without_diagnostics() {
    const FIXTURE: &str = "fixtures/recite/valid/language_pressure.recite";

    let source = fixture_source(FIXTURE);
    let parse = parse(FIXTURE, source.as_str());
    let lowered = parse.lower_source_file();

    assert_eq!(parse.syntax().text().to_string(), source);
    assert!(parse.diagnostics().is_empty(), "{:?}", parse.diagnostics());
    assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
    assert_eq!(lowered.source_file.blocks.len(), 2);
    assert_eq!(lowered.source_file.blocks[0].id.as_str(), "marche.default");
    assert_eq!(
        lowered.source_file.blocks[0]
            .default_speaker
            .as_ref()
            .map(|speaker| speaker.as_str()),
        Some("élise")
    );

    let Statement::Line(line) = &lowered.source_file.blocks[0].statements[1] else {
        panic!("expected interpolated line after the fixture comment");
    };
    assert_eq!(line.source_id.label(), Some("arrivée.interpolée"));
    assert_eq!(
        line.source_id.anchor().map(|anchor| anchor.as_str()),
        Some("55667788990011223344")
    );
    assert_eq!(
        line.source_text.text,
        "[slow]The tide is turning, {traveller_name}.[/slow]"
    );
    assert_eq!(line.interpolation_bindings.len(), 1);
    assert_eq!(line.interpolation_bindings[0].name, "traveller_name");
    assert_eq!(
        line.metadata
            .iter()
            .map(|entry| entry.key.as_str())
            .collect::<Vec<_>>(),
        ["mood", "mood"]
    );

    let Statement::Choice(choice) = &line.statements[0] else {
        panic!("expected the interpolated nested choice");
    };
    assert_eq!(
        choice.source_id.anchor().map(|anchor| anchor.as_str()),
        Some("66778899001122334455")
    );
    assert_eq!(choice.source_text.text, "Tell me about {topic}.");
    assert_eq!(choice.interpolation_bindings[0].name, "topic");

    let Statement::Line(plural) = &lowered.source_file.blocks[1].statements[0] else {
        panic!("expected plural line in the letters block");
    };
    assert_eq!(
        plural
            .plural_source_text
            .as_ref()
            .map(|text| text.text.as_str()),
        Some("You have {count} letters.")
    );
    assert_eq!(plural.interpolation_bindings[0].name, "count");
    assert_diagnostic_snapshot(&lowered.diagnostics, diagnostic_snapshot_name(FIXTURE));
    fixture_support::assert_text_snapshot(
        &lowered_fixture_summary(&lowered),
        lowered_snapshot_name(FIXTURE),
    );
}

#[test]
fn fixture_snapshots_capture_directive_boundary_diagnostics() {
    const FIXTURE: &str = "fixtures/recite/invalid/parser_directive_boundaries.recite";

    let source = fixture_source(FIXTURE);
    let parse = parse(FIXTURE, source.as_str());
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
    assert_diagnostic_snapshot(parse.diagnostics(), diagnostic_snapshot_name(FIXTURE));
}

#[test]
fn fixture_snapshots_capture_mixed_indentation_spans() {
    const FIXTURE: &str = "fixtures/recite/invalid/parser_mixed_indent.recite";

    let source = fixture_source(FIXTURE);
    let lowered = parse(FIXTURE, source.as_str()).lower_source_file();

    assert_eq!(
        line_statement(single_block(&lowered), 0).source_text.text,
        "Welcome.\nBack to the original indent."
    );
    assert_diagnostic_snapshot(&lowered.diagnostics, diagnostic_snapshot_name(FIXTURE));
}

#[test]
fn fixture_snapshots_capture_recoverable_malformed_source() {
    const FIXTURE: &str = "fixtures/recite/invalid/parser_recoverable_malformed.recite";

    let source = fixture_source(FIXTURE);
    let parse = parse(FIXTURE, source.as_str());
    let lowered = parse.lower_source_file();

    assert_eq!(parse.syntax().text().to_string(), source);
    assert_eq!(single_block(&lowered).id.as_str(), "start");
    assert_diagnostic_snapshot(&lowered.diagnostics, diagnostic_snapshot_name(FIXTURE));
}

#[test]
fn fixture_snapshots_record_current_marker_leading_prose_recovery() {
    const FIXTURE: &str = "fixtures/recite/invalid/parser_marker_leading_prose.recite";

    let source = fixture_source(FIXTURE);
    let parse = parse(FIXTURE, source.as_str());
    let lowered = parse.lower_source_file();

    assert_eq!(parse.syntax().text().to_string(), source);
    assert_diagnostic_snapshot(&lowered.diagnostics, diagnostic_snapshot_name(FIXTURE));
    assert_eq!(lowered.source_file.blocks.len(), 1);
    assert_eq!(lowered.source_file.blocks[0].statements.len(), 1);
}
