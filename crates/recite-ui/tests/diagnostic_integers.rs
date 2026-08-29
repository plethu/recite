use recite_core::{
    DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentationId, contract_for,
};
use recite_ui::{UiCatalog, UiLocale};

fn render(version: i64, expected: i64) -> String {
    let code = DiagnosticCode::new_static("RECITE_FRESH003");
    let id = DiagnosticPresentationId::new_static("diagnostic-fresh-003");
    let contract =
        contract_for(&code, &id).unwrap_or_else(|| panic!("missing contract {code}/{id}"));
    let presentation = contract
        .presentation([
            ("asset", DiagnosticArgumentValue::String("asset".to_owned())),
            ("version", DiagnosticArgumentValue::Integer(version)),
            ("expected", DiagnosticArgumentValue::Integer(expected)),
        ])
        .unwrap_or_else(|error| panic!("contract arguments: {error}"));
    UiCatalog::load(&UiLocale::default())
        .unwrap_or_else(|error| panic!("default catalog: {error}"))
        .format_presentation(&presentation)
        .unwrap_or_else(|error| panic!("English diagnostic resource: {error}"))
}

#[test]
fn integer_interpolation_preserves_i64_extremes() {
    assert_eq!(
        render(i64::MIN, i64::MAX),
        format!(
            "compiled asset 'asset' uses compiler compatibility version {}, expected {}",
            i64::MIN,
            i64::MAX
        )
    );
}

#[test]
fn exactly_representable_integers_keep_fluent_number_selectors() {
    let code = DiagnosticCode::new_static("RECITE_VALIDATE018");
    let id = DiagnosticPresentationId::new_static("diagnostic-validate-018");
    let contract =
        contract_for(&code, &id).unwrap_or_else(|| panic!("missing contract {code}/{id}"));
    let presentation = contract
        .presentation([
            (
                "function",
                DiagnosticArgumentValue::String("effect".to_owned()),
            ),
            ("expected", DiagnosticArgumentValue::Integer(1)),
            ("actual", DiagnosticArgumentValue::Integer(0)),
        ])
        .unwrap_or_else(|error| panic!("contract arguments: {error}"));
    let rendered = UiCatalog::load(&UiLocale::default())
        .unwrap_or_else(|error| panic!("default catalog: {error}"))
        .format_presentation(&presentation)
        .unwrap_or_else(|error| panic!("English diagnostic resource: {error}"));
    assert_eq!(rendered, "effect `effect` expects 1 argument, but got 0");
}
