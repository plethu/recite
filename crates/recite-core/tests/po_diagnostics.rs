#![allow(
    clippy::expect_used,
    reason = "diagnostic fixtures intentionally fail fast when expectations drift"
)]

use std::collections::BTreeMap;

use recite_core::{DiagnosticArgumentValue, DiagnosticSeverity, PoDiagnosticKind, PoDocument};

fn string(value: &str) -> DiagnosticArgumentValue {
    DiagnosticArgumentValue::String(value.to_owned())
}

fn integer(value: i64) -> DiagnosticArgumentValue {
    DiagnosticArgumentValue::Integer(value)
}

fn map(
    arguments: impl IntoIterator<Item = (&'static str, DiagnosticArgumentValue)>,
) -> BTreeMap<String, DiagnosticArgumentValue> {
    arguments
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .collect()
}

fn assert_recordable(diagnostic: &recite_core::Diagnostic) {
    assert!(diagnostic.presentation.is_some());
    assert!(diagnostic.related.is_empty());
    assert!(diagnostic.help.is_none());
    diagnostic.record().expect("PO diagnostic is recordable");
}

fn assert_diagnostic(
    source: &str,
    code: &str,
    presentation_id: &str,
    arguments: BTreeMap<String, DiagnosticArgumentValue>,
) {
    let error = PoDocument::parse(source).expect_err("malformed PO should fail");
    let diagnostic = error.diagnostic();
    assert_eq!(diagnostic.code.as_str(), code);
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);
    assert_recordable(diagnostic);
    let presentation = diagnostic
        .presentation
        .as_ref()
        .expect("PO diagnostic has a primary presentation");
    assert_eq!(presentation.id().as_str(), presentation_id);
    assert_eq!(presentation.arguments(), &arguments);
    assert!(diagnostic.explanation_presentation.is_some());
}

#[test]
fn every_finite_po_diagnostic_selector_is_emitted_with_typed_arguments() {
    assert_diagnostic(
        "unknown\n",
        "RECITE_PARSE034",
        "diagnostic-parse-034-expected-directive",
        map([]),
    );
    assert_diagnostic(
        "x-test \"x\"\n",
        "RECITE_PARSE034",
        "diagnostic-parse-034-missing-field",
        map([("field", string("msgid"))]),
    );
    assert_diagnostic(
        "msgid \"x\"\n",
        "RECITE_PARSE034",
        "diagnostic-parse-034-missing-field",
        map([("field", string("msgstr"))]),
    );
    assert_diagnostic(
        "\"x\"\n",
        "RECITE_PARSE034",
        "diagnostic-parse-034-quoted-without-field",
        map([]),
    );
    assert_diagnostic(
        "msgid \"x\" trailing\n",
        "RECITE_PARSE034",
        "diagnostic-parse-034-unexpected-trailing-text",
        map([]),
    );
    assert_diagnostic(
        "msgid \"x\n",
        "RECITE_PARSE034",
        "diagnostic-parse-034-unterminated-quoted-string",
        map([]),
    );
    assert_diagnostic(
        "msgid \"\\q\"\n",
        "RECITE_PARSE034",
        "diagnostic-parse-034-unsupported-escape",
        map([("escape", string("\\q"))]),
    );
    assert_diagnostic(
        "msgid \"x\"\nmsgctxt \"11111111111111111111\"\n",
        "RECITE_PARSE034",
        "diagnostic-parse-034-invalid-field-order",
        map([("value", string("unexpected Context"))]),
    );
    assert_diagnostic(
        "#~ msgid \"x\"\n#~ msgstr \"first\"\n#~ msgstr \"second\"\n",
        "RECITE_PARSE034",
        "diagnostic-parse-034-duplicate-field",
        map([("field", string("msgstr"))]),
    );
    assert_diagnostic(
        "msgctxt \"bad\"\nmsgid \"x\"\nmsgstr \"x\"\n",
        "RECITE_ID034",
        "diagnostic-id-034",
        map([("context", string("bad"))]),
    );
    assert_diagnostic(
        "#. source id: invalid\nmsgctxt \"11111111111111111111\"\nmsgid \"x\"\nmsgstr \"x\"\n",
        "RECITE_ID034",
        "diagnostic-id-034",
        map([("context", string("invalid"))]),
    );
    assert_diagnostic(
        "msgctxt \"11111111111111111111\"\nmsgid \"{one}\"\nmsgstr \"{other}\"\n",
        "RECITE_VALIDATE042",
        "diagnostic-validate-042",
        map([(
            "detail",
            string(
                "translation placeholders must match msgid: missing {one}; extra {other}; repetition {one} expected x1, got x0, {other} expected x0, got x1",
            ),
        )]),
    );
    assert_diagnostic(
        "msgid \"x\"\nmsgid_plural \"xs\"\n",
        "RECITE_VALIDATE043",
        "diagnostic-validate-043-contiguous-arms",
        map([]),
    );
    assert_diagnostic(
        "msgid \"x\"\nmsgid_plural \"xs\"\nmsgstr[1] \"xs\"\n",
        "RECITE_VALIDATE043",
        "diagnostic-validate-043-expected-arm",
        map([("expected", integer(0))]),
    );
    assert_diagnostic(
        "msgid \"x\"\nmsgid_plural \"xs\"\nmsgstr[a] \"xs\"\n",
        "RECITE_VALIDATE043",
        "diagnostic-validate-043-invalid-arm",
        map([("keyword", string("msgstr[a]"))]),
    );
    assert_diagnostic(
        "msgid \"\"\nmsgstr \"\"\n\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\nmsgctxt \"11111111111111111111\"\nmsgid \"x\"\nmsgid_plural \"xs\"\nmsgstr[0] \"x\"\n",
        "RECITE_VALIDATE043",
        "diagnostic-validate-043-count",
        map([("expected", integer(2)), ("actual", integer(1))]),
    );
    assert_diagnostic(
        "msgid \"\"\nmsgstr \"\"\n\"Broken header\\n\"\n",
        "RECITE_VALIDATE044",
        "diagnostic-validate-044-missing-colon",
        map([("line", string("Broken header"))]),
    );
    assert_diagnostic(
        "msgid \"\"\nmsgstr \"\"\n\"Language: en\\nLanguage: fr\\n\"\n",
        "RECITE_VALIDATE044",
        "diagnostic-validate-044-duplicate-or-empty",
        map([("key", string("Language"))]),
    );
    assert_diagnostic(
        "msgid \"\"\nmsgstr \"\"\n\"Plural-Forms: broken\\n\"\n",
        "RECITE_VALIDATE044",
        "diagnostic-validate-044-invalid-plural-forms",
        map([]),
    );
    assert_diagnostic(
        "msgid \"\"\nmsgstr \"\"\n\"Language: en\\n\"\n\nmsgid \"\"\nmsgstr \"\"\n\"Language: fr\\n\"\n",
        "RECITE_VALIDATE044",
        "diagnostic-validate-044-multiple-headers",
        map([]),
    );
    assert_diagnostic(
        "msgctxt \"11111111111111111111\"\nmsgid \"x\"\nmsgid_plural \"xs\"\nmsgstr[0] \"x\"\n",
        "RECITE_VALIDATE044",
        "diagnostic-validate-044-plural-header-required",
        map([]),
    );
    assert_diagnostic(
        "msgctxt \"11111111111111111111\"\nmsgid \"x\"\nmsgstr \"x\"\n\nmsgctxt \"11111111111111111111\"\nmsgid \"x\"\nmsgstr \"different\"\n",
        "RECITE_ID035",
        "diagnostic-id-035",
        map([
            ("context", string("11111111111111111111")),
            ("source_text", string("x")),
        ]),
    );
}

#[test]
fn po_diagnostic_spans_preserve_raw_fields_comments_crlf_and_scalar_columns() {
    let duplicate = concat!(
        "#~ msgid \"x\"\n",
        "#~ msgstr \"first\"\n",
        "#~ \"continued\"\n",
        "#~ msgstr \"second\"\n",
        "#~ \"continuation\"\n",
    );
    let duplicate_error = PoDocument::parse(duplicate).expect_err("duplicate obsolete field");
    assert!(matches!(
        duplicate_error.kind(),
        PoDiagnosticKind::DuplicateField(field) if field == "msgstr"
    ));
    assert_eq!(duplicate_error.diagnostic().span.start.line(), 4);
    assert_eq!(
        duplicate_error
            .diagnostic()
            .span
            .end
            .as_ref()
            .map(|position| position.line()),
        Some(5)
    );
    assert_recordable(duplicate_error.diagnostic());

    let invalid_order = "msgid \"x\"\nmsgctxt \"11111111111111111111\"\n";
    let order_error = PoDocument::parse(invalid_order).expect_err("invalid field order");
    assert_eq!(order_error.diagnostic().span.start.line(), 2);
    assert_eq!(order_error.diagnostic().span.start.column(), 1);
    assert_eq!(
        order_error
            .diagnostic()
            .span
            .end
            .as_ref()
            .map(|position| position.line()),
        Some(2)
    );
    assert_recordable(order_error.diagnostic());

    let source_id = "#. source id: invalid\r\nmsgctxt \"11111111111111111111\"\r\nmsgid \"x\"\r\nmsgstr \"x\"\r\n";
    let source_id_error = PoDocument::parse(source_id).expect_err("invalid extracted source ID");
    assert_eq!(source_id_error.diagnostic().span.start.line(), 1);
    assert_eq!(source_id_error.diagnostic().span.start.column(), 1);
    assert_eq!(
        source_id_error
            .diagnostic()
            .span
            .end
            .as_ref()
            .map(|position| position.line()),
        Some(1)
    );
    assert_recordable(source_id_error.diagnostic());

    let scalar = "msgctxt \"é\"\r\nmsgid \"x\"\r\nmsgstr \"x\"\r\n";
    let scalar_error = PoDocument::parse(scalar).expect_err("invalid scalar context");
    assert_eq!(scalar_error.diagnostic().span.start.line(), 1);
    assert_eq!(scalar_error.diagnostic().span.start.column(), 9);
    assert_eq!(
        scalar_error
            .diagnostic()
            .span
            .end
            .as_ref()
            .map(|position| position.column()),
        Some(12)
    );
    assert_recordable(scalar_error.diagnostic());
}
