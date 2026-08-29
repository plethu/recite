use recite_core::{PoDiagnosticKind, PoDocument};

#[test]
fn adjacent_obsolete_comments_remain_with_the_obsolete_record() {
    let source = concat!(
        "msgctxt \"11111111111111111111\"\n",
        "msgid \"Active\"\n",
        "msgstr \"Actif\"\n",
        "#~ #. obsolete note\n",
        "#~ #: old.po:7\n",
        "#~ msgid \"Gone\"\n",
        "#~ msgstr \"Gone\"\n",
    );
    let document = PoDocument::parse(source).expect("adjacent obsolete record parses");
    assert_eq!(document.entries().len(), 2);
    assert!(!document.entries()[0].is_obsolete());
    assert_eq!(document.entries()[0].comments().len(), 0);
    assert!(document.entries()[1].is_obsolete());
    assert_eq!(document.entries()[1].comments().len(), 2);
    assert!(
        document.entries()[1]
            .comments()
            .iter()
            .all(|comment| comment.is_obsolete())
    );
    assert_eq!(document.source(), source);
}

#[test]
fn msgfmt_adjacent_obsolete_comments_do_not_obsolete_active_entries() {
    let source = include_str!("fixtures/po-adjacent.po");
    let document = PoDocument::parse(source).expect("msgfmt-checked fixture parses");
    assert!(!document.entries()[4].is_obsolete());
    assert!(document.entries()[5].is_obsolete());
    assert_eq!(document.entries()[5].comments().len(), 2);
    assert!(
        document.entries()[5]
            .comments()
            .iter()
            .all(|comment| comment.is_obsolete())
    );
}

#[test]
fn deeply_nested_plural_expressions_report_structured_errors() {
    let depth = 512;
    let cases = [
        (
            "parentheses",
            format!("{}n{}", "(".repeat(depth), ")".repeat(depth)),
        ),
        ("unary", format!("{}n", "!".repeat(depth))),
        (
            "ternary",
            format!("{}n{}", "n?".repeat(depth), ":n".repeat(depth)),
        ),
    ];
    for (kind, expression) in cases {
        let source = format!(
            "msgid \"\"\nmsgstr \"\"\n\"Plural-Forms: nplurals=2; plural={expression};\\n\"\n"
        );
        let error = match PoDocument::parse(source) {
            Ok(_) => panic!("excessive {kind} plural nesting was accepted"),
            Err(error) => error,
        };
        assert!(matches!(
            error.kind(),
            PoDiagnosticKind::InvalidHeader(detail) if detail.contains("Plural-Forms")
        ));
        assert_eq!(error.diagnostic().code.as_str(), "RECITE_VALIDATE044");
    }
}

#[test]
fn duplicate_header_records_report_structured_errors() {
    let source = concat!(
        "msgid \"\"\nmsgstr \"\"\n\"Language: en\\n\"\n\n",
        "msgid \"\"\nmsgstr \"\"\n\"Language: fr\\n\"\n",
    );
    let error = PoDocument::parse(source).expect_err("duplicate header records are rejected");
    assert!(matches!(
        error.kind(),
        PoDiagnosticKind::InvalidHeader(detail) if detail.contains("multiple header records")
    ));
    assert_eq!(error.diagnostic().code.as_str(), "RECITE_VALIDATE044");
    assert_eq!(error.line(), 5);
}
