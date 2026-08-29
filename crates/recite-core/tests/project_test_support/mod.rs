use std::collections::BTreeMap;

use recite_core::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticSeverity, explain_diagnostic_code,
};

pub(crate) fn string(value: &str) -> DiagnosticArgumentValue {
    DiagnosticArgumentValue::String(value.to_owned())
}

pub(crate) fn assert_diagnostic(
    diagnostic: &Diagnostic,
    code: &str,
    presentation_id: &str,
    arguments: &[(&str, DiagnosticArgumentValue)],
) {
    assert_eq!(diagnostic.code.as_str(), code);
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);
    assert_recordable(diagnostic);
    assert!(explain_diagnostic_code(&diagnostic.code).is_some());
    let presentation = diagnostic
        .presentation
        .as_ref()
        .unwrap_or_else(|| panic!("structured presentation"));
    assert_eq!(presentation.id().as_str(), presentation_id);
    let expected = arguments
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(presentation.arguments(), &expected);
}

pub(crate) fn assert_recordable(diagnostic: &Diagnostic) {
    assert_eq!(diagnostic.related, Vec::new());
    assert!(diagnostic.help.is_none());
    assert!(diagnostic.explanation_presentation.is_none());
    if let Err(error) = diagnostic.record() {
        panic!("recordable structured diagnostic: {error:?}");
    }
}
