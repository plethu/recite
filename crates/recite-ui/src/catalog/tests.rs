use super::diagnostics::ResourceRegistry;
use super::*;
use crate::args::fluent_args;
use crate::{Client, ResourceId, ResourceSpec, UiArg, UiArgType, UiContract};

#[test]
fn typed_float_and_boolean_values_resolve_from_a_valid_resource() {
    let source = "weight = weight={$weight}\nenabled = enabled={$enabled}\n";
    let contract = UiContract::new(
        vec![
            ResourceSpec::new(ResourceId::new("weight").expect("id"))
                .argument("weight", UiArgType::Float)
                .client(Client::Cli),
            ResourceSpec::new(ResourceId::new("enabled").expect("id"))
                .argument("enabled", UiArgType::Boolean)
                .client(Client::Cli),
        ],
        vec![crate::ClientSpec::new(Client::Cli, "CLI", true)],
    );
    contract.validate(source).expect("typed resource is valid");

    let resource = FluentResource::try_new(source.to_owned()).expect("resource");
    let locale = "en-US".parse().expect("locale");
    let mut bundle = FluentBundle::new(vec![locale]);
    bundle.set_use_isolating(false);
    bundle
        .add_resource(resource)
        .expect("resource conflict free");
    let args = UiArgs::from([
        ("weight".to_owned(), UiArg::Float(1.5)),
        ("enabled".to_owned(), UiArg::Boolean(true)),
    ]);
    let mut errors = Vec::new();
    let weight = bundle.format_pattern(
        bundle
            .get_message("weight")
            .expect("weight")
            .value()
            .expect("value"),
        Some(&fluent_args(&args)),
        &mut errors,
    );
    assert_eq!(weight, "weight=1.5");
    let enabled = bundle.format_pattern(
        bundle
            .get_message("enabled")
            .expect("enabled")
            .value()
            .expect("value"),
        Some(&fluent_args(&args)),
        &mut errors,
    );
    assert_eq!(enabled, "enabled=true");
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn resource_registry_uses_one_owned_lookup_path() {
    let contract = UiContract::default();
    let registry = ResourceRegistry::from_contract(&contract);
    let id = ResourceId::new("diagnostic-parse-001-meaning").expect("resource ID");
    assert!(registry.get(&id).is_some());
    assert_eq!(registry.len(), contract.resources.len());
    assert!(
        registry
            .get(&ResourceId::new("diagnostic-not-inventory").expect("resource ID"))
            .is_none()
    );
}
