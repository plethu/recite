use recite_core::{
    DiagnosticArgumentType, DiagnosticAuxiliaryPresentationContract, DiagnosticExplanation,
    DiagnosticPresentation, DiagnosticPresentationContract, known_diagnostic_explanations,
    migrated_diagnostic_auxiliary_presentation_contracts,
    migrated_diagnostic_presentation_contracts,
};

use super::{Client, ResourceId, ResourceSpec};
use crate::UiArgType;

/// Build active localised diagnostic resources and explanation inventory from
/// the core's stable diagnostic families.
pub(super) fn resource_specs() -> Vec<ResourceSpec> {
    migrated_diagnostic_presentation_contracts()
        .flat_map(resource_spec_for_contract)
        .chain(
            migrated_diagnostic_auxiliary_presentation_contracts()
                .flat_map(resource_spec_for_auxiliary_contract),
        )
        .chain(known_diagnostic_explanations().flat_map(resource_specs_for_explanation))
        .collect()
}

#[allow(
    clippy::expect_used,
    reason = "contract presentation IDs are validated as static resource IDs"
)]
fn resource_spec_for_contract(
    contract: &'static DiagnosticPresentationContract,
) -> Vec<ResourceSpec> {
    let mut resource = ResourceSpec::new(
        ResourceId::new(contract.presentation_id().as_str()).expect("valid resource ID"),
    );
    for argument in contract.arguments() {
        resource = resource.argument(argument.name(), argument_type(argument.argument_type()));
    }
    vec![
        resource
            .client(Client::Cli)
            .client(Client::Tui)
            .client(Client::Lsp)
            .client(Client::VsCode)
            .client(Client::VsCodium),
    ]
}

#[allow(
    clippy::expect_used,
    reason = "contract presentation IDs are validated as static resource IDs"
)]
fn resource_spec_for_auxiliary_contract(
    contract: &'static DiagnosticAuxiliaryPresentationContract,
) -> Vec<ResourceSpec> {
    let mut resource = ResourceSpec::new(
        ResourceId::new(contract.presentation_id().as_str()).expect("valid resource ID"),
    );
    for argument in contract.arguments() {
        resource = resource.argument(argument.name(), argument_type(argument.argument_type()));
    }
    vec![
        resource
            .client(Client::Cli)
            .client(Client::Tui)
            .client(Client::Lsp)
            .client(Client::VsCode)
            .client(Client::VsCodium),
    ]
}

fn argument_type(argument_type: DiagnosticArgumentType) -> UiArgType {
    match argument_type {
        DiagnosticArgumentType::String => UiArgType::String,
        DiagnosticArgumentType::Integer => UiArgType::Integer,
        DiagnosticArgumentType::Float => UiArgType::Float,
        DiagnosticArgumentType::Boolean => UiArgType::Boolean,
    }
}

fn resource_specs_for_explanation(
    explanation: &'static DiagnosticExplanation,
) -> Vec<ResourceSpec> {
    let presentation = explanation.presentation();
    std::iter::once(presentation.meaning)
        .chain(presentation.common_causes)
        .chain(presentation.remediation)
        .map(|presentation| resource_spec(&presentation))
        .collect()
}

/// The one explicit compatibility resource for producers that still only
/// provide the legacy English `message` field. It is not diagnostic inventory
/// coverage and must not be used as a translated primary presentation.
#[allow(
    clippy::expect_used,
    reason = "the compatibility adapter ID is a fixed valid resource ID"
)]
pub(super) fn legacy_resource_spec() -> ResourceSpec {
    ResourceSpec::new(
        ResourceId::new(crate::LEGACY_DIAGNOSTIC_RESOURCE)
            .expect("legacy diagnostic resource ID is valid"),
    )
    .argument("message", UiArgType::String)
    .client(Client::Cli)
    .client(Client::Tui)
    .client(Client::Lsp)
    .client(Client::VsCode)
    .client(Client::VsCodium)
}

#[allow(
    clippy::expect_used,
    reason = "core generates IDs in the same validated grammar"
)]
fn resource_spec(presentation: &DiagnosticPresentation) -> ResourceSpec {
    ResourceSpec::new(ResourceId::new(presentation.id().as_str()).expect("valid resource ID"))
        .client(Client::Cli)
        .client(Client::Tui)
        .client(Client::Lsp)
        .client(Client::VsCode)
        .client(Client::VsCodium)
}
