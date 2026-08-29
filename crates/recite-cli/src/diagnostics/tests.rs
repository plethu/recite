use recite_core::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentation,
    DiagnosticPresentationId, DiagnosticRelatedPresentation, SourcePosition, SourceSpan,
    auxiliary_contract_for, contract_for,
};

use super::report_diagnostics;
use crate::i18n::{Messages, UiLocale};

fn locale(value: &str) -> unic_langid::LanguageIdentifier {
    value.parse().expect("test locale")
}

fn point(file: &str, line: u32, column: u32) -> SourceSpan {
    SourceSpan::point(
        file,
        SourcePosition::new(line, column).expect("test source position"),
    )
}

fn primary_diagnostic() -> Diagnostic {
    let code = DiagnosticCode::new_static("RECITE_PARSE001");
    let presentation_id = DiagnosticPresentationId::new_static("diagnostic-parse-001");
    let contract = contract_for(&code, &presentation_id).expect("parse presentation contract");
    Diagnostic::error_from_contract(
        contract,
        "compatibility primary",
        point("dialogue/main.recite", 1, 1),
        std::iter::empty::<(&str, DiagnosticArgumentValue)>(),
    )
    .expect("diagnostic arguments match contract")
}

fn auxiliary_presentation(id: &'static str) -> DiagnosticPresentation {
    let contract = auxiliary_contract_for(&DiagnosticPresentationId::new_static(id))
        .expect("auxiliary presentation contract");
    contract
        .presentation(std::iter::empty::<(&str, DiagnosticArgumentValue)>())
        .expect("auxiliary arguments match contract")
}

fn messages_with_overrides(requested: &str, overrides: &[(&str, &str)]) -> Messages {
    let resource = overrides.iter().fold(
        recite_ui::DEFAULT_RESOURCE.to_owned(),
        |resource, (from, to)| resource.replace(from, to),
    );
    Messages::from_resources(
        locale(requested),
        [
            (locale("en-US"), recite_ui::DEFAULT_RESOURCE.to_owned()),
            (locale(requested), resource),
        ],
    )
    .expect("messages load")
}

#[test]
fn report_diagnostics_uses_selected_locale_for_primary_text() {
    let messages = messages_with_overrides(
        "fr-FR",
        &[(
            "diagnostic-parse-001 = expected a Recite statement header or indented prose",
            "diagnostic-parse-001 = localized primary",
        )],
    );
    let diagnostics = [primary_diagnostic()];
    let mut output = Vec::new();

    assert_eq!(
        report_diagnostics(&mut output, &messages, diagnostics.iter()).expect("report"),
        1
    );
    assert_eq!(
        String::from_utf8(output).expect("UTF-8 output"),
        "error RECITE_PARSE001 dialogue/main.recite:1:1 localized primary\n"
    );
}

#[test]
fn report_diagnostics_uses_compatibility_text_when_primary_is_unavailable() {
    let messages = Messages::load(&UiLocale::default()).expect("messages load");
    let diagnostic = Diagnostic::error(
        DiagnosticCode::new_static("RECITE_PARSE001"),
        "compatibility fallback",
        point("dialogue/main.recite", 1, 1),
    )
    .with_presentation(DiagnosticPresentation::new(
        DiagnosticPresentationId::new_static("diagnostic-not-in-catalog"),
    ));
    let mut output = Vec::new();

    report_diagnostics(&mut output, &messages, std::iter::once(&diagnostic)).expect("report");

    assert_eq!(
        String::from_utf8(output).expect("UTF-8 output"),
        "error RECITE_PARSE001 dialogue/main.recite:1:1 compatibility fallback\n"
    );
}

#[test]
fn report_diagnostics_preserves_shared_related_and_help_order() {
    let messages = messages_with_overrides(
        "fr-FR",
        &[
            (
                "diagnostic-parse-001 = expected a Recite statement header or indented prose",
                "diagnostic-parse-001 = localized primary",
            ),
            (
                "diagnostic-id-003-related = first localisable ID is here",
                "diagnostic-id-003-related = first related",
            ),
            (
                "diagnostic-id-004-related = first localisable ID is here",
                "diagnostic-id-004-related = second related",
            ),
            (
                "diagnostic-id-003-help = rename one of the duplicate localisable IDs",
                "diagnostic-id-003-help = shared help",
            ),
        ],
    );
    let diagnostic = primary_diagnostic()
        .with_related_presentations([
            DiagnosticRelatedPresentation::new(
                point("dialogue/first.recite", 2, 3),
                auxiliary_presentation("diagnostic-id-003-related"),
            ),
            DiagnosticRelatedPresentation::new(
                point("dialogue/second.recite", 8, 13),
                auxiliary_presentation("diagnostic-id-004-related"),
            ),
        ])
        .with_help_presentation(auxiliary_presentation("diagnostic-id-003-help"));
    let mut output = Vec::new();

    report_diagnostics(&mut output, &messages, std::iter::once(&diagnostic)).expect("report");

    assert_eq!(
        String::from_utf8(output).expect("UTF-8 output"),
        concat!(
            "error RECITE_PARSE001 dialogue/main.recite:1:1 localized primary\n",
            "  related dialogue/first.recite:2:3 first related\n",
            "  related dialogue/second.recite:8:13 second related\n",
            "  help: shared help\n",
        )
    );
}
