use recite_core::{SourceRecovery, ast::Statement};
use recite_parser::parse;

#[test]
fn unicode_prose_preserves_internal_blanks_and_trims_boundary_blanks() {
    for newline in ["\n", "\r\n"] {
        let source = [
            ":: start default",
            "> prompt@5e0925cd041f2f6df9e2",
            "",
            "  Café ☕  ",
            "  ",
            "  日本語 🌿",
            "",
            "  ",
            "  ? continue@8d398d18dbde0d7303c2",
            "",
            "    進みます。",
            "    ",
            "    -> END",
        ]
        .join(newline);
        let parsed = parse("prose.recite", &source);
        assert_eq!(parsed.syntax().text().to_string(), source);
        let lowered = parsed.lower_source_file();
        assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
        assert_eq!(lowered.recovery, SourceRecovery::complete());
        let Statement::Line(line) = &lowered.source_file.blocks[0].statements[0] else {
            panic!("expected prompt line");
        };
        assert_eq!(line.source_text.text, "Café ☕  \n\n日本語 🌿");
        assert_eq!(line.source_text.span.start.line(), 4);
        assert_eq!(line.source_text.span.start.column(), 3);
        let Statement::Choice(choice) = &line.statements[0] else {
            panic!("expected nested choice");
        };
        assert_eq!(choice.source_text.text, "進みます。");
        assert_eq!(choice.source_text.span.start.line(), 11);
        assert_eq!(choice.source_text.span.start.column(), 5);
        assert!(choice.target.is_some());
    }
}

#[test]
fn tab_indented_prose_preserves_unicode_and_plural_spans() {
    let source = concat!(
        ":: start default\n",
        "> letters@5e0925cd041f2f6df9e2\n",
        "\tUne lettre égarée.\n",
        "\t| Des lettres égarées.\n",
        "\n",
        "-> END\n",
    );
    let lowered = parse("tabs.recite", source).lower_source_file();
    assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
    let Statement::Line(line) = &lowered.source_file.blocks[0].statements[0] else {
        panic!("expected plural line");
    };
    assert_eq!(line.source_text.text, "Une lettre égarée.");
    assert_eq!(line.source_text.span.start.line(), 3);
    assert_eq!(line.source_text.span.start.column(), 2);
    let plural = line
        .plural_source_text
        .as_ref()
        .expect("plural source text");
    assert_eq!(plural.text, "Des lettres égarées.");
    assert_eq!(plural.span.start.line(), 4);
    assert_eq!(plural.span.start.column(), 4);
}

#[test]
fn mixed_indent_recovery_keeps_only_the_accepted_unicode_prose() {
    let source = concat!(
        ":: start default\n",
        "> prompt@5e0925cd041f2f6df9e2\n",
        "  Café ☕\n",
        "    診断する行\n",
        "  Déjà vu.\n",
    );
    let lowered = parse("recovery.recite", source).lower_source_file();
    assert_eq!(lowered.diagnostics.len(), 1);
    let diagnostic = &lowered.diagnostics[0];
    assert_eq!(diagnostic.code.as_str(), "RECITE_PARSE007");
    assert_eq!(diagnostic.span.file, "recovery.recite");
    assert_eq!(diagnostic.span.start.line(), 4);
    assert_eq!(diagnostic.span.start.column(), 5);
    assert!(!lowered.recovery.ast_structure());
    let Statement::Line(line) = &lowered.source_file.blocks[0].statements[0] else {
        panic!("expected recovered line");
    };
    assert_eq!(line.source_text.text, "Café ☕\nDéjà vu.");
    assert_eq!(line.source_text.span.start.line(), 3);
    assert_eq!(line.source_text.span.start.column(), 3);
}
