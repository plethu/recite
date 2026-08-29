use recite_core::{
    DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentationId, auxiliary_contract_for,
    contract_for,
};
use recite_ui::{UiCatalog, UiLocale};

struct Case {
    code: &'static str,
    id: &'static str,
    arguments: Vec<(&'static str, DiagnosticArgumentValue)>,
    expected: String,
}

fn string(value: &str) -> DiagnosticArgumentValue {
    DiagnosticArgumentValue::String(value.to_owned())
}

fn integer(value: i64) -> DiagnosticArgumentValue {
    DiagnosticArgumentValue::Integer(value)
}

fn case(
    code: &'static str,
    id: &'static str,
    arguments: impl IntoIterator<Item = (&'static str, DiagnosticArgumentValue)>,
    expected: impl Into<String>,
) -> Case {
    Case {
        code,
        id,
        arguments: arguments.into_iter().collect(),
        expected: expected.into(),
    }
}

fn render(catalog: &UiCatalog, case: Case) -> String {
    let code = DiagnosticCode::new_static(case.code);
    let id = DiagnosticPresentationId::new_static(case.id);
    let contract =
        contract_for(&code, &id).unwrap_or_else(|| panic!("missing contract {code}/{id}"));
    let presentation = contract
        .presentation(case.arguments)
        .unwrap_or_else(|error| panic!("contract arguments for {id}: {error}"));
    catalog
        .format_presentation(&presentation)
        .unwrap_or_else(|error| panic!("resource {id}: {error}"))
}

fn cases() -> Vec<Case> {
    vec![
        case(
            "RECITE_PARSE034",
            "diagnostic-parse-034-expected-directive",
            [],
            "expected PO directive",
        ),
        case(
            "RECITE_PARSE034",
            "diagnostic-parse-034-expected-quoted-string",
            [],
            "expected quoted PO string",
        ),
        case(
            "RECITE_PARSE034",
            "diagnostic-parse-034-missing-field",
            [("field", string("msgstr"))],
            "entry is missing msgstr",
        ),
        case(
            "RECITE_PARSE034",
            "diagnostic-parse-034-duplicate-field",
            [("field", string("msgctxt"))],
            "duplicate PO field msgctxt",
        ),
        case(
            "RECITE_PARSE034",
            "diagnostic-parse-034-quoted-without-field",
            [],
            "quoted continuation without a PO field",
        ),
        case(
            "RECITE_PARSE034",
            "diagnostic-parse-034-unexpected-trailing-text",
            [],
            "unexpected text after quoted PO string",
        ),
        case(
            "RECITE_PARSE034",
            "diagnostic-parse-034-unterminated-quoted-string",
            [],
            "unterminated quoted PO string",
        ),
        case(
            "RECITE_PARSE034",
            "diagnostic-parse-034-unsupported-escape",
            [("escape", string("\\q"))],
            "unsupported PO escape \\q",
        ),
        case(
            "RECITE_PARSE034",
            "diagnostic-parse-034-invalid-field-order",
            [("value", string("unexpected PluralTranslation(12)"))],
            "invalid PO field order: unexpected PluralTranslation(12)",
        ),
        case(
            "RECITE_ID034",
            "diagnostic-id-034",
            [("context", string("ctx@bad"))],
            "invalid stable PO context `ctx@bad`",
        ),
        case(
            "RECITE_ID035",
            "diagnostic-id-035",
            [
                ("context", string("scene@anchor")),
                ("source_text", string("Hello {name}")),
            ],
            "duplicate PO catalogue key: context `scene@anchor` and msgid `Hello {name}`",
        ),
        case(
            "RECITE_VALIDATE042",
            "diagnostic-validate-042",
            [("detail", string("missing {name}; extra {other}"))],
            "PO placeholder mismatch: missing {name}; extra {other}",
        ),
        case(
            "RECITE_VALIDATE043",
            "diagnostic-validate-043-contiguous-arms",
            [],
            "plural entries require contiguous msgstr[N] arms",
        ),
        case(
            "RECITE_VALIDATE043",
            "diagnostic-validate-043-expected-arm",
            [("expected", integer(9_007_199_254_740_993))],
            "expected msgstr[9007199254740993]",
        ),
        case(
            "RECITE_VALIDATE043",
            "diagnostic-validate-043-requires-plural-source",
            [],
            "msgstr[N] requires msgid_plural",
        ),
        case(
            "RECITE_VALIDATE043",
            "diagnostic-validate-043-count",
            [
                ("expected", integer(i64::MIN)),
                ("actual", integer(i64::MAX)),
            ],
            format!(
                "header declares {} plural arms but entry has {}",
                i64::MIN,
                i64::MAX
            ),
        ),
        case(
            "RECITE_VALIDATE043",
            "diagnostic-validate-043-invalid-arm",
            [("keyword", string("msgstr[abc]"))],
            "invalid plural arm `msgstr[abc]`",
        ),
        case(
            "RECITE_VALIDATE044",
            "diagnostic-validate-044-multiple-headers",
            [],
            "PO document contains multiple header records",
        ),
        case(
            "RECITE_VALIDATE044",
            "diagnostic-validate-044-missing-colon",
            [("line", string("Plural-Forms nplurals=2"))],
            "header line `Plural-Forms nplurals=2` lacks `:`",
        ),
        case(
            "RECITE_VALIDATE044",
            "diagnostic-validate-044-duplicate-or-empty",
            [("key", string("Content-Type"))],
            "duplicate or empty header `Content-Type`",
        ),
        case(
            "RECITE_VALIDATE044",
            "diagnostic-validate-044-invalid-plural-forms",
            [],
            "Plural-Forms must declare positive nplurals and a plural expression",
        ),
        case(
            "RECITE_VALIDATE044",
            "diagnostic-validate-044-invalid-plural-rule",
            [("detail", string("plural expression divided by zero"))],
            "Plural-Forms rule is unusable: plural expression divided by zero",
        ),
        case(
            "RECITE_VALIDATE044",
            "diagnostic-validate-044-plural-header-required",
            [],
            "active plural entries require Plural-Forms with nplurals and plural",
        ),
        case(
            "RECITE_PROJECT001",
            "diagnostic-project-001",
            [("detail", string("invalid TOML at line 12"))],
            "malformed project manifest: invalid TOML at line 12",
        ),
        case(
            "RECITE_PROJECT002",
            "diagnostic-project-002",
            [("scene_id", string("opening-λ"))],
            "duplicate scene id 'opening-λ'",
        ),
        case(
            "RECITE_PROJECT003",
            "diagnostic-project-003",
            [
                ("scene_id", string("intro/λ")),
                ("asset", string("dialogue/one.msgpack")),
            ],
            "scene 'intro/λ' references missing compiled asset 'dialogue/one.msgpack'",
        ),
        case(
            "RECITE_PROJECT004",
            "diagnostic-project-004",
            [
                ("scene_id", string("intro")),
                ("block", string("missing-block")),
            ],
            "scene 'intro' references unknown block 'missing-block'",
        ),
        case(
            "RECITE_PROJECT005",
            "diagnostic-project-005",
            [("scene_id", string("intro"))],
            "scene 'intro' must declare at least one participant",
        ),
        case(
            "RECITE_PROJECT006",
            "diagnostic-project-006",
            [
                ("asset", string("dialogue.msgpack")),
                ("source", string("dialogue.recite")),
            ],
            "compiled asset 'dialogue.msgpack' references missing source 'dialogue.recite'",
        ),
        case(
            "RECITE_PROJECT007",
            "diagnostic-project-007",
            [
                ("asset", string("dialogue.msgpack")),
                ("version", integer(9_007_199_254_740_993)),
            ],
            "compiled asset 'dialogue.msgpack' uses unsupported format version 9007199254740993",
        ),
        case(
            "RECITE_PROJECT007",
            "diagnostic-project-007-malformed",
            [
                ("scene_id", string("intro")),
                ("asset", string("dialogue.msgpack")),
                ("detail", string("bad top-level shape")),
            ],
            "scene 'intro' references malformed compiled asset 'dialogue.msgpack': bad top-level shape",
        ),
        case(
            "RECITE_PROJECT008",
            "diagnostic-project-008",
            [
                ("scene_id", string("intro")),
                ("participant", string("narrator")),
            ],
            "scene 'intro' references unknown participant 'narrator'",
        ),
        case(
            "RECITE_PROJECT008",
            "diagnostic-project-008-compiled-asset",
            [
                ("scene_id", string("intro")),
                ("participant", string("narrator")),
                ("asset", string("dialogue.msgpack")),
            ],
            "scene 'intro' participant 'narrator' is not present in compiled asset 'dialogue.msgpack'",
        ),
        case(
            "RECITE_FRESH001",
            "diagnostic-fresh-001",
            [
                ("asset", string("dialogue.msgpack")),
                ("source", string("dialogue.recite")),
            ],
            "compiled asset 'dialogue.msgpack' is stale for source 'dialogue.recite'",
        ),
        case(
            "RECITE_FRESH002",
            "diagnostic-fresh-002",
            [("asset", string("dialogue.msgpack"))],
            "compiled asset 'dialogue.msgpack' has a stale schema fingerprint",
        ),
        case(
            "RECITE_FRESH003",
            "diagnostic-fresh-003",
            [
                ("asset", string("dialogue.msgpack")),
                ("version", integer(i64::MIN)),
                ("expected", integer(i64::MAX)),
            ],
            format!(
                "compiled asset 'dialogue.msgpack' uses compiler compatibility version {}, expected {}",
                i64::MIN,
                i64::MAX
            ),
        ),
    ]
}

#[test]
fn every_remaining_resource_preserves_its_compatibility_message() {
    let catalog = UiCatalog::load(&UiLocale::default())
        .unwrap_or_else(|error| panic!("default catalog: {error}"));
    let cases = cases();
    assert_eq!(cases.len(), 36);
    for case in cases {
        let expected = case.expected.clone();
        assert_eq!(render(&catalog, case), expected);
    }

    let auxiliary = auxiliary_contract_for(&DiagnosticPresentationId::new_static(
        "diagnostic-project-002-related",
    ))
    .unwrap_or_else(|| panic!("missing duplicate-scene related contract"));
    let presentation = auxiliary
        .presentation(std::iter::empty::<(&str, DiagnosticArgumentValue)>())
        .unwrap_or_else(|error| panic!("auxiliary arguments: {error}"));
    assert_eq!(
        catalog
            .format_presentation(&presentation)
            .unwrap_or_else(|error| panic!("auxiliary resource: {error}")),
        "first scene with this id"
    );
}
