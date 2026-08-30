use std::collections::BTreeMap;

use recite_core::{
    known_diagnostic_explanations, migrated_diagnostic_auxiliary_presentation_contracts,
    migrated_diagnostic_presentation_contracts,
};
use recite_ui::{Client, ClientSpec, LEGACY_DIAGNOSTIC_RESOURCE, UiArgType, UiContract};
use serde::Deserialize;

#[test]
fn diagnostic_contract_matches_registry_and_resources() {
    let contract = UiContract::default();
    let diagnostic_resources = contract
        .resources
        .iter()
        .filter(|resource| {
            resource.id.as_str().starts_with("diagnostic-")
                && resource.id.as_str() != LEGACY_DIAGNOSTIC_RESOURCE
        })
        .collect::<Vec<_>>();

    #[derive(Deserialize)]
    struct Inventory {
        diagnostic_presentation_ids: Vec<String>,
    }
    let inventory: Inventory = toml::from_str(include_str!("../resources/inventory.toml"))
        .expect("valid resource inventory");
    let expected_default_ids = known_diagnostic_explanations()
        .map(|explanation| explanation.default_code_presentation_id().to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        inventory.diagnostic_presentation_ids, expected_default_ids,
        "future default-code diagnostic IDs must remain a complete explicit registry"
    );

    let migrated_presentation_ids = migrated_diagnostic_presentation_contracts()
        .map(|contract| contract.presentation_id().to_string())
        .collect::<Vec<_>>();
    let legacy_count = contract
        .resources
        .iter()
        .filter(|resource| resource.id.as_str() == LEGACY_DIAGNOSTIC_RESOURCE)
        .count();
    assert_eq!(legacy_count, 1, "exactly one legacy diagnostic adapter");
    let legacy = contract
        .resources
        .iter()
        .find(|resource| resource.id.as_str() == LEGACY_DIAGNOSTIC_RESOURCE)
        .expect("legacy diagnostic adapter resource");
    assert_eq!(
        legacy.arguments,
        BTreeMap::from([("message".to_owned(), UiArgType::String)])
    );

    let diagnostic_ids = diagnostic_resources
        .iter()
        .map(|resource| resource.id.as_str().to_owned())
        .collect::<Vec<_>>();
    let expected_explanation_ids = known_diagnostic_explanations()
        .flat_map(|explanation| {
            let presentation = explanation.presentation();
            let mut ids = vec![presentation.meaning.id().to_string()];
            ids.extend(
                presentation
                    .common_causes
                    .iter()
                    .chain(presentation.remediation.iter())
                    .map(|reference| reference.id().to_string()),
            );
            ids
        })
        .collect::<Vec<_>>();
    let expected_active_ids = migrated_presentation_ids
        .into_iter()
        .chain(
            migrated_diagnostic_auxiliary_presentation_contracts()
                .map(|contract| contract.presentation_id().to_string()),
        )
        .chain(expected_explanation_ids)
        .collect::<Vec<_>>();
    assert_eq!(
        diagnostic_ids, expected_active_ids,
        "active diagnostic inventory must match migrated primaries and structured explanation slots"
    );

    let diagnostic_contract = UiContract::new(
        contract
            .resources
            .iter()
            .filter(|resource| resource.id.as_str().starts_with("diagnostic-"))
            .cloned()
            .collect(),
        vec![ClientSpec::new(Client::Cli, "CLI", true)],
    );
    diagnostic_contract
        .validate(include_str!("../resources/diagnostics.ftl"))
        .expect("diagnostic resource file is complete independently");

    let invalid_document_key = contract
        .resources
        .iter()
        .find(|resource| resource.id.as_str() == "diagnostic-config-117")
        .expect("invalid document key resource is registered");
    assert_eq!(
        invalid_document_key.arguments,
        BTreeMap::from([("detail".to_owned(), UiArgType::String)])
    );
    assert_eq!(
        invalid_document_key.clients,
        [Client::Cli, Client::Tui, Client::Lsp]
            .into_iter()
            .collect()
    );
}
