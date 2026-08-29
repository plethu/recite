use super::super::DiagnosticArgumentSpec;
use super::super::DiagnosticAuxiliaryPresentationContract;
use super::super::DiagnosticPresentationContract;
use super::super::DiagnosticPresentationContractRegistryError;
use super::validation::validate_contract_registry;

#[test]
fn primary_and_auxiliary_ids_are_checked_as_one_namespace() {
    const NO_ARGUMENTS: &[DiagnosticArgumentSpec] = &[];
    let primary = DiagnosticPresentationContract::new(
        "RECITE_TEST001",
        "diagnostic-test-shared",
        NO_ARGUMENTS,
    );
    let auxiliary =
        DiagnosticAuxiliaryPresentationContract::new("diagnostic-test-shared", NO_ARGUMENTS);

    assert!(matches!(
        validate_contract_registry([&primary], [&auxiliary]),
        Err(DiagnosticPresentationContractRegistryError::DuplicatePresentationId(id))
            if id == "diagnostic-test-shared"
    ));
}
