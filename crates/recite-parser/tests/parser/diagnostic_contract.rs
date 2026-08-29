use super::*;

#[test]
fn parser_diagnostics_are_structured_and_recordable() {
    let source = concat!(
        "oops\n",
        ":: tavern\n",
        "!\n",
        "! blocking\n",
        ":if @\n",
        ":case tired\n",
        "? ask@a74221348f0e47548c59 if trust_gte(hazel, rhea, 3)\n",
        "  What's the news?\n",
    );

    let lowered = parse("dialogue/diagnostics.recite", source).lower_source_file();
    assert_eq!(
        lowered
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        [
            "RECITE_PARSE001",
            "RECITE_PARSE002",
            "RECITE_PARSE012",
            "RECITE_PARSE012",
            "RECITE_PARSE013",
            "RECITE_PARSE016",
            "RECITE_PARSE018",
        ]
    );

    for diagnostic in &lowered.diagnostics {
        assert_recordable_diagnostic(diagnostic);
        let record = diagnostic.record().expect("migrated parser record");
        assert_eq!(record.code, diagnostic.code);
        assert_eq!(record.span, diagnostic.span);
        assert!(diagnostic.related.is_empty());
        assert!(diagnostic.help.is_none());
        assert!(diagnostic.presentation.is_some());
        assert!(diagnostic.explanation_presentation.is_some());
    }
    assert_deterministic_en_us_compatibility_fallback(
        &lowered.diagnostics[0],
        "expected a Recite statement header or indented prose",
    );

    let missing_mode = &lowered.diagnostics[2];
    assert_eq!(
        missing_mode.presentation.as_ref().unwrap().id().as_str(),
        "diagnostic-parse-012"
    );
    assert_eq!(
        missing_mode
            .presentation
            .as_ref()
            .unwrap()
            .arguments()
            .get("reason"),
        Some(&recite_core::DiagnosticArgumentValue::String(
            "missing_mode".to_owned()
        ))
    );

    let unexpected_character = &lowered.diagnostics[4];
    assert_eq!(
        unexpected_character
            .presentation
            .as_ref()
            .unwrap()
            .id()
            .as_str(),
        "diagnostic-parse-013-unexpected-character"
    );
    assert_eq!(
        unexpected_character
            .presentation
            .as_ref()
            .unwrap()
            .arguments()
            .get("character"),
        Some(&recite_core::DiagnosticArgumentValue::String(
            "@".to_owned()
        ))
    );
}

fn assert_deterministic_en_us_compatibility_fallback(
    diagnostic: &recite_core::Diagnostic,
    expected: &str,
) {
    assert_eq!(
        diagnostic
            .record()
            .expect("compatibility fallback record")
            .compatibility_message(),
        Some(expected)
    );
}

#[test]
fn parser_static_and_dynamic_contracts_match_central_registry() {
    let source = concat!(":: tavern\n", "! blocking\n", "! immediate play_sfx(@)\n",);
    let diagnostics = parse("dialogue/diagnostics.recite", source)
        .lower_source_file()
        .diagnostics;

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].code.as_str(), "RECITE_PARSE012");
    assert_eq!(diagnostics[1].code.as_str(), "RECITE_PARSE012");
    assert_eq!(
        diagnostics[0].presentation.as_ref().unwrap().id().as_str(),
        "diagnostic-parse-012"
    );
    assert_eq!(
        diagnostics[1].presentation.as_ref().unwrap().id().as_str(),
        "diagnostic-parse-012-unexpected-character"
    );
    assert_eq!(
        diagnostics[0].presentation.as_ref().unwrap().arguments()["reason"],
        recite_core::DiagnosticArgumentValue::String("expected_function_call".to_owned())
    );
    assert_eq!(
        diagnostics[1].presentation.as_ref().unwrap().arguments(),
        &std::collections::BTreeMap::from([(
            "character".to_owned(),
            recite_core::DiagnosticArgumentValue::String("@".to_owned()),
        )])
    );
    for diagnostic in &diagnostics {
        assert_recordable_diagnostic(diagnostic);
    }
}

fn assert_recordable_diagnostic(diagnostic: &recite_core::Diagnostic) {
    assert!(diagnostic.presentation.is_some());
    assert!(diagnostic.related.is_empty());
    assert!(diagnostic.help.is_none());
    diagnostic
        .record()
        .expect("parser diagnostic is recordable");
}

#[test]
fn unexpected_character_presentation_uses_debug_escape_tokens() {
    let source = concat!(":: tavern\n", ":if '\n");
    let diagnostics = parse("dialogue/diagnostics.recite", source)
        .lower_source_file()
        .diagnostics;

    assert_eq!(diagnostics.len(), 1);
    assert_recordable_diagnostic(&diagnostics[0]);
    assert_eq!(
        diagnostics[0].presentation.as_ref().unwrap().id().as_str(),
        "diagnostic-parse-013-unexpected-character"
    );
    assert_eq!(
        diagnostics[0].presentation.as_ref().unwrap().arguments()["character"],
        recite_core::DiagnosticArgumentValue::String("\\'".to_owned())
    );
}
