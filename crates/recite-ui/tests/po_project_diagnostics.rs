use recite_core::{
    DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentationId, contract_for,
};
use recite_ui::{UiCatalog, UiLocale};

fn render(
    catalog: &UiCatalog,
    code: &str,
    id: &str,
    arguments: impl IntoIterator<Item = (&'static str, DiagnosticArgumentValue)>,
) -> String {
    let contract = contract_for(
        &DiagnosticCode::new(code).unwrap_or_else(|error| panic!("valid diagnostic code: {error}")),
        &DiagnosticPresentationId::new(id)
            .unwrap_or_else(|error| panic!("valid presentation ID: {error}")),
    )
    .unwrap_or_else(|| panic!("missing contract {code}/{id}"));
    let presentation = contract
        .presentation(arguments)
        .unwrap_or_else(|error| panic!("contract arguments: {error}"));
    catalog
        .format_presentation(&presentation)
        .unwrap_or_else(|error| panic!("English diagnostic resource: {error}"))
}

#[test]
fn po_presentations_preserve_distinct_english_compatibility_messages() {
    let catalog = UiCatalog::load(&UiLocale::default())
        .unwrap_or_else(|error| panic!("default catalog: {error}"));
    for (id, expected) in [
        (
            "diagnostic-parse-034-expected-directive",
            "expected PO directive",
        ),
        (
            "diagnostic-parse-034-expected-quoted-string",
            "expected quoted PO string",
        ),
        (
            "diagnostic-parse-034-quoted-without-field",
            "quoted continuation without a PO field",
        ),
        (
            "diagnostic-parse-034-unexpected-trailing-text",
            "unexpected text after quoted PO string",
        ),
        (
            "diagnostic-parse-034-unterminated-quoted-string",
            "unterminated quoted PO string",
        ),
        (
            "diagnostic-validate-043-contiguous-arms",
            "plural entries require contiguous msgstr[N] arms",
        ),
        (
            "diagnostic-validate-043-requires-plural-source",
            "msgstr[N] requires msgid_plural",
        ),
        (
            "diagnostic-validate-044-multiple-headers",
            "PO document contains multiple header records",
        ),
        (
            "diagnostic-validate-044-invalid-plural-forms",
            "Plural-Forms must declare positive nplurals and a plural expression",
        ),
        (
            "diagnostic-validate-044-plural-header-required",
            "active plural entries require Plural-Forms with nplurals and plural",
        ),
    ] {
        let code = if id.starts_with("diagnostic-parse") {
            "RECITE_PARSE034"
        } else if id.starts_with("diagnostic-validate-043") {
            "RECITE_VALIDATE043"
        } else {
            "RECITE_VALIDATE044"
        };
        assert_eq!(render(&catalog, code, id, []), expected);
    }
    assert_eq!(
        render(
            &catalog,
            "RECITE_PARSE034",
            "diagnostic-parse-034-missing-field",
            [(
                "field",
                DiagnosticArgumentValue::String("msgstr".to_owned())
            )],
        ),
        "entry is missing msgstr"
    );
    assert_eq!(
        render(
            &catalog,
            "RECITE_PARSE034",
            "diagnostic-parse-034-unsupported-escape",
            [("escape", DiagnosticArgumentValue::String("\\q".to_owned()))],
        ),
        "unsupported PO escape \\q"
    );
    assert_eq!(
        render(
            &catalog,
            "RECITE_ID035",
            "diagnostic-id-035",
            [
                (
                    "context",
                    DiagnosticArgumentValue::String("scene@anchor".to_owned())
                ),
                (
                    "source_text",
                    DiagnosticArgumentValue::String("Hello".to_owned())
                ),
            ],
        ),
        "duplicate PO catalogue key: context `scene@anchor` and msgid `Hello`"
    );
}

#[test]
fn project_and_freshness_presentations_preserve_cli_messages() {
    let catalog = UiCatalog::load(&UiLocale::default())
        .unwrap_or_else(|error| panic!("default catalog: {error}"));
    assert_eq!(
        render(
            &catalog,
            "RECITE_PROJECT001",
            "diagnostic-project-001",
            [(
                "detail",
                DiagnosticArgumentValue::String("invalid TOML".to_owned())
            )],
        ),
        "malformed project manifest: invalid TOML"
    );
    assert_eq!(
        render(
            &catalog,
            "RECITE_CONFIG117",
            "diagnostic-config-117",
            [(
                "detail",
                DiagnosticArgumentValue::String(
                    "dir\\start.recite: document key must use slash separators".to_owned(),
                ),
            )],
        ),
        "project source has an invalid document key: dir\\start.recite: document key must use slash separators"
    );
    assert_eq!(
        render(
            &catalog,
            "RECITE_PROJECT003",
            "diagnostic-project-003",
            [
                (
                    "scene_id",
                    DiagnosticArgumentValue::String("intro".to_owned())
                ),
                (
                    "asset",
                    DiagnosticArgumentValue::String("dialogue.msgpack".to_owned())
                ),
            ],
        ),
        "scene 'intro' references missing compiled asset 'dialogue.msgpack'"
    );
    assert_eq!(
        render(
            &catalog,
            "RECITE_PROJECT007",
            "diagnostic-project-007-malformed",
            [
                (
                    "scene_id",
                    DiagnosticArgumentValue::String("intro".to_owned())
                ),
                (
                    "asset",
                    DiagnosticArgumentValue::String("dialogue.msgpack".to_owned())
                ),
                (
                    "detail",
                    DiagnosticArgumentValue::String("bad top-level shape".to_owned())
                ),
            ],
        ),
        "scene 'intro' references malformed compiled asset 'dialogue.msgpack': bad top-level shape"
    );
    assert_eq!(
        render(
            &catalog,
            "RECITE_FRESH003",
            "diagnostic-fresh-003",
            [
                (
                    "asset",
                    DiagnosticArgumentValue::String("dialogue.msgpack".to_owned())
                ),
                ("version", DiagnosticArgumentValue::Integer(1)),
                ("expected", DiagnosticArgumentValue::Integer(0)),
            ],
        ),
        "compiled asset 'dialogue.msgpack' uses compiler compatibility version 1, expected 0"
    );
}
