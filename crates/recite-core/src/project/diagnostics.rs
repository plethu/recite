use crate::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentationId,
    DiagnosticRelatedPresentation, SourceSpan, auxiliary_contract_for, contract_for,
};

#[allow(
    clippy::expect_used,
    reason = "this helper owns project diagnostic contract lookup and argument validation"
)]
pub(super) fn project_diagnostic<I, K>(
    code: &DiagnosticCode,
    presentation_id: &'static str,
    message: impl Into<String>,
    span: SourceSpan,
    arguments: I,
) -> Diagnostic
where
    I: IntoIterator<Item = (K, DiagnosticArgumentValue)>,
    K: Into<String>,
{
    let contract = contract_for(code, &DiagnosticPresentationId::new_static(presentation_id))
        .expect("project diagnostic contract is registered");
    Diagnostic::error_from_contract(contract, message, span, arguments)
        .expect("project diagnostic arguments match their central contract")
}

#[allow(
    clippy::expect_used,
    reason = "this helper owns project related-presentation lookup and argument validation"
)]
pub(super) fn related_presentation(
    span: SourceSpan,
    presentation_id: &'static str,
) -> DiagnosticRelatedPresentation {
    let contract = auxiliary_contract_for(&DiagnosticPresentationId::new_static(presentation_id))
        .expect("project related diagnostic contract is registered");
    let presentation = contract
        .presentation(std::iter::empty::<(&str, DiagnosticArgumentValue)>())
        .expect("project related diagnostic arguments match their contract");
    DiagnosticRelatedPresentation::new(span, presentation)
}
