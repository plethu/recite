use super::{
    DiagnosticAuxiliaryPresentationContract, DiagnosticPresentationContract, compiler, freshness,
    parser, po, project, schema,
};
use crate::diagnostic_presentation_record::is_valid_argument_name;
use std::collections::BTreeSet;

/// A broken invariant in the first-party diagnostic presentation registry.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DiagnosticPresentationContractRegistryError {
    DuplicatePresentationId(String),
    DuplicateArgument {
        presentation_id: String,
        name: String,
    },
    InvalidArgumentName {
        presentation_id: String,
        name: String,
    },
}

impl std::fmt::Display for DiagnosticPresentationContractRegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicatePresentationId(id) => {
                write!(formatter, "duplicate diagnostic presentation ID `{id}`")
            }
            Self::DuplicateArgument {
                presentation_id,
                name,
            } => write!(
                formatter,
                "diagnostic presentation `{presentation_id}` declares duplicate argument `{name}`"
            ),
            Self::InvalidArgumentName {
                presentation_id,
                name,
            } => write!(
                formatter,
                "diagnostic presentation `{presentation_id}` declares invalid argument name `{name}`"
            ),
        }
    }
}

impl std::error::Error for DiagnosticPresentationContractRegistryError {}

/// Validate a set of producer contracts before adding it to a first-party
/// presentation registry. Presentation IDs are globally unique within the
/// set and each exact presentation signature has no duplicate argument names.
pub fn validate_diagnostic_presentation_contracts<'a>(
    contracts: impl IntoIterator<Item = &'a DiagnosticPresentationContract>,
) -> Result<(), DiagnosticPresentationContractRegistryError> {
    let mut presentation_ids = BTreeSet::new();
    for contract in contracts {
        let presentation_id = contract.presentation_id().to_string();
        if !presentation_ids.insert(presentation_id.clone()) {
            return Err(
                DiagnosticPresentationContractRegistryError::DuplicatePresentationId(
                    presentation_id,
                ),
            );
        }
        validate_arguments(contract.presentation_id().as_str(), contract.arguments())?;
    }
    Ok(())
}

/// Validate the first-party contract registry before adding another producer
/// family. IDs are globally unique and each exact presentation signature has
/// no duplicate argument names.
pub fn validate_migrated_diagnostic_presentation_contracts()
-> Result<(), DiagnosticPresentationContractRegistryError> {
    let primary = parser::contracts()
        .chain(po::contracts())
        .chain(compiler::contracts())
        .chain(project::contracts())
        .chain(freshness::contracts())
        .chain(schema::contracts())
        .collect::<Vec<_>>();
    let auxiliary =
        super::migrated_diagnostic_auxiliary_presentation_contracts().collect::<Vec<_>>();
    validate_contract_registry(primary.iter().copied(), auxiliary.iter().copied())
}

pub(super) fn validate_contract_registry<'a>(
    primary: impl IntoIterator<Item = &'a DiagnosticPresentationContract>,
    auxiliary: impl IntoIterator<Item = &'a DiagnosticAuxiliaryPresentationContract>,
) -> Result<(), DiagnosticPresentationContractRegistryError> {
    let primary = primary.into_iter().collect::<Vec<_>>();
    validate_diagnostic_presentation_contracts(primary.iter().copied())?;
    let auxiliary = auxiliary.into_iter().collect::<Vec<_>>();
    validate_auxiliary_diagnostic_presentation_contracts(auxiliary.iter().copied())?;
    let mut presentation_ids = primary
        .iter()
        .map(|contract| contract.presentation_id().to_string())
        .collect::<BTreeSet<_>>();
    for contract in auxiliary {
        let presentation_id = contract.presentation_id().to_string();
        if !presentation_ids.insert(presentation_id.clone()) {
            return Err(
                DiagnosticPresentationContractRegistryError::DuplicatePresentationId(
                    presentation_id,
                ),
            );
        }
    }
    Ok(())
}

/// Validate auxiliary contracts independently from primary contracts.
pub fn validate_auxiliary_diagnostic_presentation_contracts<'a>(
    contracts: impl IntoIterator<Item = &'a DiagnosticAuxiliaryPresentationContract>,
) -> Result<(), DiagnosticPresentationContractRegistryError> {
    let mut presentation_ids = BTreeSet::new();
    for contract in contracts {
        let presentation_id = contract.presentation_id().to_string();
        if !presentation_ids.insert(presentation_id.clone()) {
            return Err(
                DiagnosticPresentationContractRegistryError::DuplicatePresentationId(
                    presentation_id,
                ),
            );
        }
        validate_arguments(contract.presentation_id().as_str(), contract.arguments())?;
    }
    Ok(())
}

fn validate_arguments(
    presentation_id: &str,
    arguments: &[super::super::DiagnosticArgumentSpec],
) -> Result<(), DiagnosticPresentationContractRegistryError> {
    let mut argument_names = BTreeSet::new();
    for argument in arguments {
        let name = argument.name().to_owned();
        if !is_valid_argument_name(&name) {
            return Err(
                DiagnosticPresentationContractRegistryError::InvalidArgumentName {
                    presentation_id: presentation_id.to_owned(),
                    name,
                },
            );
        }
        if !argument_names.insert(name.clone()) {
            return Err(
                DiagnosticPresentationContractRegistryError::DuplicateArgument {
                    presentation_id: presentation_id.to_owned(),
                    name,
                },
            );
        }
    }
    Ok(())
}
