use recite_core::{PoDiagnosticKind, PoDocument};

fn catalogue(source_text: &str, translation: &str) -> String {
    format!("msgctxt \"11111111111111111111\"\nmsgid \"{source_text}\"\nmsgstr \"{translation}\"\n")
}

#[test]
fn translated_markup_allows_reordered_prose_and_round_trips_losslessly() {
    let source = concat!(
        "# translator note\n",
        "#. source id: line@11111111111111111111\n",
        "x-editor \"kept\"\n",
        "msgctxt \"11111111111111111111\"\n",
        "msgid \"[slow]Hello [em]world[/em][/slow]\"\n",
        "msgstr \"[slow]Monde [em]bonjour[/em][/slow]\"\n",
    );
    let document = PoDocument::parse_with_path("dialogue.po", source)
        .expect("reordered translated prose keeps authored markup");
    assert_eq!(document.source(), source);
    assert_eq!(
        document.entries()[0].translation(),
        Some("[slow]Monde [em]bonjour[/em][/slow]")
    );
    assert_eq!(
        document.entries()[0].unknown_fields()[0].keyword(),
        "x-editor"
    );
    assert_eq!(
        document.entries()[0].comments()[0].text(),
        "translator note"
    );
}

#[test]
fn translated_markup_allows_reordered_tagged_spans() {
    let document = PoDocument::parse(catalogue(
        "[slow tone=calm]Hello[/slow] [slow tone=warm]world[/slow]",
        "[slow tone=warm]monde[/slow] [slow tone=calm]bonjour[/slow]",
    ))
    .expect("whole tagged spans may be reordered");
    assert_eq!(
        document.entries()[0].translation(),
        Some("[slow tone=warm]monde[/slow] [slow tone=calm]bonjour[/slow]")
    );
}

#[test]
fn translated_markup_rejects_missing_required_tags() {
    let error = PoDocument::parse(catalogue(
        "[slow]Hello [em]world[/em][/slow]",
        "Hello [em]monde[/em]",
    ))
    .expect_err("missing source tags must be rejected");
    assert!(matches!(
        error.kind(),
        PoDiagnosticKind::MarkupMissingTag(tag) if tag == "slow"
    ));
    assert_eq!(error.diagnostic().code.as_str(), "RECITE_VALIDATE049");
    assert_eq!(
        error
            .diagnostic()
            .presentation
            .as_ref()
            .expect("missing-tag presentation")
            .id()
            .as_str(),
        "diagnostic-validate-049"
    );
    assert_eq!(error.diagnostic().span.start.line(), 3);
}

#[test]
fn translated_markup_rejects_new_tags() {
    let error = PoDocument::parse(catalogue(
        "[slow]Hello[/slow]",
        "[slow]Bonjour[/slow] [ghost]now[/ghost]",
    ))
    .expect_err("new source tags must be rejected");
    assert!(matches!(
        error.kind(),
        PoDiagnosticKind::MarkupUnknownTag(tag) if tag == "ghost"
    ));
    assert_eq!(error.diagnostic().code.as_str(), "RECITE_VALIDATE048");
    assert_eq!(
        error
            .diagnostic()
            .presentation
            .as_ref()
            .expect("new-tag presentation")
            .id()
            .as_str(),
        "diagnostic-validate-048"
    );
}

#[test]
fn translated_markup_rejects_unbalanced_tags() {
    let error = PoDocument::parse(catalogue(
        "[slow]Hello [em]world[/em][/slow]",
        "[slow]Bonjour [em]monde[/slow][/em]",
    ))
    .expect_err("unbalanced translated tags must be rejected");
    assert!(matches!(
        error.kind(),
        PoDiagnosticKind::MarkupUnbalancedTag(detail) if detail.contains("expected closing tag for `em`")
    ));
    assert_eq!(error.diagnostic().code.as_str(), "RECITE_VALIDATE023");
}

#[test]
fn translated_markup_rejects_forbidden_attribute_changes() {
    let error = PoDocument::parse(catalogue(
        "[slow mood=calm]Hello[/slow]",
        "[slow mood=angry]Bonjour[/slow]",
    ))
    .expect_err("translated tag attributes must remain source-stable");
    assert!(matches!(
        error.kind(),
        PoDiagnosticKind::MarkupAttributeChange { tag, expected, actual }
            if tag == "slow" && expected == "mood=calm" && actual == "mood=angry"
    ));
    assert_eq!(error.diagnostic().code.as_str(), "RECITE_VALIDATE047");
}

#[test]
fn translated_markup_rejects_nested_same_name_attribute_swaps() {
    let error = PoDocument::parse(catalogue(
        "[mark role=outer][mark role=inner]text[/mark][/mark]",
        "[mark role=inner][mark role=outer]texte[/mark][/mark]",
    ))
    .expect_err("nested same-name attributes must remain structurally matched");
    assert!(matches!(
        error.kind(),
        PoDiagnosticKind::MarkupAttributeChange { tag, expected, actual }
            if tag == "mark" && expected == "role=outer" && actual == "role=inner"
    ));
}
