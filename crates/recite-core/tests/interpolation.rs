use recite_core::{
    PlaceholderSyntaxKind, decode_interpolation_text, extract_placeholder_names,
    extract_placeholder_occurrences, validate_translation_placeholders,
};

#[test]
fn placeholder_validation_preserves_repetition() {
    assert!(validate_translation_placeholders("{name} {name}", "{name}").is_err());
    assert_eq!(
        extract_placeholder_occurrences(r"\{name\} {name}").unwrap(),
        ["name"]
    );
}

#[test]
fn escaped_braces_decode_only_at_runtime_boundary() {
    assert_eq!(
        decode_interpolation_text(r"literal \{brace\}"),
        "literal {brace}"
    );
}

#[test]
fn literal_unicode_text_and_unrelated_escapes_stay_unchanged() {
    for source in [
        "Crème brûlée · 🦊\n[slow]À bientôt.[/slow]",
        r"A \path, \字, two \\slashes and a trailing \",
        "",
    ] {
        assert_eq!(decode_interpolation_text(source), source);
        assert!(extract_placeholder_names(source).unwrap().is_empty());
        assert!(extract_placeholder_occurrences(source).unwrap().is_empty());
    }
}

#[test]
fn placeholder_scans_preserve_brace_errors_and_escaped_literals() {
    for (source, kind) in [
        ("À bientôt }", PlaceholderSyntaxKind::UnescapedClosingBrace),
        ("À bientôt {", PlaceholderSyntaxKind::Unterminated),
        (
            "À bientôt {Bad}",
            PlaceholderSyntaxKind::InvalidName("Bad".to_owned()),
        ),
    ] {
        assert_eq!(extract_placeholder_names(source).unwrap_err().kind(), &kind);
        assert_eq!(
            extract_placeholder_occurrences(source).unwrap_err().kind(),
            &kind,
        );
    }
    let source = r"À bientôt \{literal\} {name} {name}";
    assert_eq!(
        extract_placeholder_names(source)
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>(),
        ["name"],
    );
    assert_eq!(
        extract_placeholder_occurrences(source).unwrap(),
        ["name", "name"]
    );
    assert_eq!(
        decode_interpolation_text(source),
        "À bientôt {literal} {name} {name}"
    );
}
