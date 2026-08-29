use super::{
    DiagnosticAuxiliaryPresentationContract, DiagnosticPresentationContract, compiler, config,
    freshness, parser, po, project, schema,
};
use std::sync::OnceLock;

mod lookup;
mod validation;

#[cfg(test)]
mod tests;

pub use lookup::{auxiliary_contract_for, contract_for, contracts_for_code, presentation_for};
pub use validation::{
    DiagnosticPresentationContractRegistryError,
    validate_auxiliary_diagnostic_presentation_contracts,
    validate_diagnostic_presentation_contracts,
    validate_migrated_diagnostic_presentation_contracts,
};

static REGISTRY_VALIDATED: OnceLock<()> = OnceLock::new();

/// All first-party diagnostic contracts currently migrated to the structured
/// producer boundary.
pub fn migrated_diagnostic_presentation_contracts()
-> impl Iterator<Item = &'static DiagnosticPresentationContract> {
    ensure_registry_validated();
    parser::contracts()
        .chain(po::contracts())
        .chain(compiler::contracts())
        .chain(config::contracts())
        .chain(project::contracts())
        .chain(freshness::contracts())
        .chain(schema::contracts())
}

/// Structured contracts for related-span and help presentations emitted by
/// migrated first-party producers. These are deliberately separate from
/// primary diagnostic contracts because they have no diagnostic code of their
/// own.
pub fn migrated_diagnostic_auxiliary_presentation_contracts()
-> impl Iterator<Item = &'static DiagnosticAuxiliaryPresentationContract> {
    compiler::auxiliary_contracts().chain(project::auxiliary_contracts())
}

fn ensure_registry_validated() {
    REGISTRY_VALIDATED.get_or_init(|| {
        assert!(
            validation::validate_migrated_diagnostic_presentation_contracts().is_ok(),
            "first-party diagnostic presentation registry invariants are broken"
        );
    });
}
