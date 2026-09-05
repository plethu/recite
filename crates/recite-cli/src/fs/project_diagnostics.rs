use recite_core::{Diagnostic, DiagnosticArgumentValue, DiagnosticPresentationId, contract_for};

#[allow(
    clippy::expect_used,
    reason = "this helper owns the CLI project diagnostic contract invariant"
)]
pub(super) fn project_diagnostic<I, K>(
    code: &recite_core::DiagnosticCode,
    presentation_id: &'static str,
    message: impl Into<String>,
    span: recite_core::SourceSpan,
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
