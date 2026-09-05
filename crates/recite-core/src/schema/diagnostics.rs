use crate::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentationId, SourceSpan,
    contract_for,
};

/// Construct a schema diagnostic from the central first-party contract.
///
/// Schema lowering is intentionally infallible at this internal boundary. The
/// contract pair and argument list are both owned by this crate, so a failure
/// here indicates a programmer error in the static diagnostic registry rather
/// than malformed project content.
#[allow(
    clippy::expect_used,
    reason = "this helper owns schema diagnostic contract lookup and argument validation"
)]
pub(crate) fn schema_diagnostic<I, K>(
    code: DiagnosticCode,
    presentation_id: &'static str,
    message: impl Into<String>,
    span: SourceSpan,
    arguments: I,
) -> Diagnostic
where
    I: IntoIterator<Item = (K, DiagnosticArgumentValue)>,
    K: Into<String>,
{
    let presentation_id = DiagnosticPresentationId::new_static(presentation_id);
    let contract =
        contract_for(&code, &presentation_id).expect("schema diagnostic contract is registered");
    Diagnostic::error_from_contract(contract, message, span, arguments)
        .expect("schema diagnostic arguments match their contract")
}

#[cfg(test)]
mod tests;
