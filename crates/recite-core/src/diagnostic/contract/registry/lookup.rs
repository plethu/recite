use super::{DiagnosticAuxiliaryPresentationContract, DiagnosticPresentationContract};
use crate::diagnostic::DiagnosticCode;
use crate::diagnostic_argument::DiagnosticArgumentValue;
use crate::diagnostic_presentation::DiagnosticPresentationId;

/// Resolve a related/help contract by its stable presentation ID.
#[must_use]
pub fn auxiliary_contract_for(
    presentation_id: &DiagnosticPresentationId,
) -> Option<&'static DiagnosticAuxiliaryPresentationContract> {
    // Auxiliary producers are resolved at runtime, so they must observe the
    // same cross-family primary/auxiliary validation as primary producers.
    // This keeps a bad auxiliary registry from being hidden behind a lookup.
    super::ensure_registry_validated();
    super::migrated_diagnostic_auxiliary_presentation_contracts()
        .find(|contract| contract.presentation_id() == presentation_id)
}

/// Resolve all migrated presentation contracts for one machine-facing code.
pub fn contracts_for_code(
    code: &DiagnosticCode,
) -> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    super::migrated_diagnostic_presentation_contracts()
        .filter(move |contract| contract.code() == code)
}

/// Resolve one exact code/presentation pair from the migrated registry.
#[must_use]
pub fn contract_for(
    code: &DiagnosticCode,
    presentation_id: &DiagnosticPresentationId,
) -> Option<&'static DiagnosticPresentationContract> {
    let mut found = None;
    for contract in contracts_for_code(code) {
        if contract.presentation_id() != presentation_id {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some(contract);
    }
    found
}

/// Build a presentation only when its code and resource ID form a registered
/// pair and its named arguments satisfy that pair's exact signature.
pub fn presentation_for<I, K>(
    code: &DiagnosticCode,
    presentation_id: &DiagnosticPresentationId,
    arguments: I,
) -> Result<
    crate::diagnostic_presentation_record::DiagnosticPresentation,
    crate::DiagnosticPresentationError,
>
where
    I: IntoIterator<Item = (K, DiagnosticArgumentValue)>,
    K: Into<String>,
{
    let Some(contract) = contract_for(code, presentation_id) else {
        return Err(crate::DiagnosticPresentationError::UnknownContract {
            code: code.to_string(),
            presentation_id: presentation_id.to_string(),
        });
    };
    contract.presentation(arguments)
}
