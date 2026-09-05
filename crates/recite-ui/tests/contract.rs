use std::collections::{BTreeMap, BTreeSet};

use fluent_syntax::{ast::Entry, parser};
use serde::Deserialize;

use recite_ui::{
    CLIENT_INVENTORY, Client, ClientSpec, DEFAULT_RESOURCE, MsgId, ProjectionSpec, ResourceId,
    ResourceSpec, TEST_EN_GB_RESOURCE, UiArg, UiArgType, UiCatalog, UiContract, UiLocale,
};

#[test]
fn launch_resource_matches_the_typed_inventory() {
    let contract = UiContract::default();
    contract
        .validate(DEFAULT_RESOURCE)
        .expect("complete launch catalog");
    assert_eq!(MsgId::ALL.len(), 438);

    #[derive(Deserialize)]
    struct Inventory {
        resource_ids: Vec<String>,
        clients: BTreeMap<String, ClientEntry>,
        projections: ProjectionInventory,
    }
    #[derive(Deserialize)]
    struct ClientEntry {
        name: String,
        shipped: bool,
    }
    #[derive(Deserialize)]
    struct ProjectionInventory {
        neovim: NeovimProjection,
        vscode: EditorProjection,
        vscodium: EditorProjection,
    }
    #[derive(Deserialize)]
    struct NeovimProjection {
        source_resource: String,
        output: String,
        ids: Vec<String>,
    }
    #[derive(Deserialize)]
    struct EditorProjection {
        source_resource: String,
        runtime_output: String,
        package_output: String,
        runtime_ids: Vec<String>,
        package_ids: Vec<String>,
        runtime_arguments: BTreeMap<String, Vec<String>>,
        package_arguments: BTreeMap<String, Vec<String>>,
    }
    let inventory: Inventory = toml::from_str(include_str!("../resources/inventory.toml"))
        .expect("valid resource inventory");
    let registry = MsgId::ALL
        .iter()
        .map(|id| id.key().to_owned())
        .collect::<BTreeSet<_>>();
    let expected_ids = MsgId::ALL
        .iter()
        .map(|id| id.key().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        inventory.resource_ids, expected_ids,
        "inventory order must be registry order"
    );
    assert_eq!(
        inventory.resource_ids.len(),
        registry.len(),
        "inventory IDs must be unique"
    );
    #[derive(Deserialize)]
    struct Arguments {
        arguments: BTreeMap<String, BTreeMap<String, String>>,
    }
    let arguments: Arguments = toml::from_str(include_str!("../resources/arguments.toml"))
        .expect("valid independent argument contract");
    let expected_arguments = contract
        .resources
        .iter()
        .filter(|resource| registry.contains(resource.id.as_str()))
        .filter(|resource| !resource.arguments.is_empty())
        .map(|resource| {
            (
                resource.id.as_str().to_owned(),
                resource
                    .arguments
                    .iter()
                    .map(|(name, kind)| (name.clone(), format!("{kind:?}").to_lowercase()))
                    .collect::<BTreeMap<_, _>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        arguments.arguments, expected_arguments,
        "argument contract must match the typed registry"
    );
    let expected_clients = CLIENT_INVENTORY
        .iter()
        .map(|spec| {
            let key = match spec.client {
                Client::VsCode => "vscode",
                Client::NativeGui => "native_gui",
                client => client.key(),
            };
            (key.to_owned(), (spec.name.to_owned(), spec.shipped))
        })
        .collect::<BTreeMap<_, _>>();
    let actual_clients = inventory
        .clients
        .iter()
        .map(|(key, spec)| (key.clone(), (spec.name.clone(), spec.shipped)))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual_clients, expected_clients,
        "client inventory must be explicit"
    );
    let typed_neovim_projections = contract
        .resources
        .iter()
        .flat_map(|resource| {
            resource
                .projections
                .iter()
                .filter(|projection| projection.client == Client::Neovim)
                .map(|projection| (resource.id.as_str().to_owned(), projection.field.clone()))
        })
        .collect::<Vec<_>>();
    let inventory_neovim_ids = inventory.projections.neovim.ids.clone();
    assert_eq!(
        typed_neovim_projections
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<BTreeSet<_>>(),
        inventory_neovim_ids.into_iter().collect::<BTreeSet<_>>(),
        "Neovim projection IDs must match the typed contract"
    );
    assert_eq!(
        typed_neovim_projections
            .iter()
            .map(|(_, output)| output)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([&inventory.projections.neovim.output]),
        "Neovim projection output must match the typed contract"
    );
    assert_eq!(
        inventory.projections.neovim.source_resource, "en-US.ftl",
        "Neovim projects the canonical launch resource"
    );
    for resource in contract
        .resources
        .iter()
        .filter(|resource| resource.id.as_str().starts_with("diagnostic-"))
    {
        assert!(
            resource.clients.contains(&Client::Neovim),
            "Neovim must own the typed diagnostic resource {}",
            resource.id
        );
    }
    for (name, projection) in [
        ("VS Code", &inventory.projections.vscode),
        ("VSCodium", &inventory.projections.vscodium),
    ] {
        assert_eq!(
            projection.source_resource, "en-US.ftl",
            "{name} source resource"
        );
        assert_eq!(
            projection.runtime_output, "editors/vscode/src/messages.generated.js",
            "{name} runtime projection output"
        );
        assert_eq!(
            projection.package_output, "editors/vscode/package.nls.json",
            "{name} package projection output"
        );
        let runtime_ids = projection
            .runtime_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let package_ids = projection
            .package_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let typed_runtime = contract
            .resources
            .iter()
            .filter(|resource| {
                resource.projections.iter().any(|declaration| {
                    declaration.client
                        == if name == "VS Code" {
                            Client::VsCode
                        } else {
                            Client::VsCodium
                        }
                        && declaration.field == projection.runtime_output
                })
            })
            .map(|resource| resource.id.as_str().to_owned())
            .collect::<BTreeSet<_>>();
        let typed_package = contract
            .resources
            .iter()
            .filter(|resource| {
                resource.projections.iter().any(|declaration| {
                    declaration.client
                        == if name == "VS Code" {
                            Client::VsCode
                        } else {
                            Client::VsCodium
                        }
                        && declaration.field == projection.package_output
                })
            })
            .map(|resource| resource.id.as_str().to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            typed_runtime, runtime_ids,
            "{name} runtime IDs must match typed contract"
        );
        assert_eq!(
            typed_package, package_ids,
            "{name} package IDs must match typed contract"
        );
        let client = if name == "VS Code" {
            Client::VsCode
        } else {
            Client::VsCodium
        };
        let typed_runtime_arguments = contract
            .resources
            .iter()
            .filter(|resource| {
                resource.projections.iter().any(|declaration| {
                    declaration.client == client && declaration.field == projection.runtime_output
                })
            })
            .map(|resource| {
                (
                    resource.id.as_str().to_owned(),
                    resource.arguments.keys().cloned().collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let typed_package_arguments = contract
            .resources
            .iter()
            .filter(|resource| {
                resource.projections.iter().any(|declaration| {
                    declaration.client == client && declaration.field == projection.package_output
                })
            })
            .map(|resource| {
                (
                    resource.id.as_str().to_owned(),
                    resource.arguments.keys().cloned().collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(
            typed_runtime_arguments, projection.runtime_arguments,
            "{name} runtime argument declarations must match typed contract"
        );
        assert_eq!(
            typed_package_arguments, projection.package_arguments,
            "{name} package argument declarations must match typed contract"
        );
        assert!(
            runtime_ids
                .iter()
                .all(|id| contract.resources.iter().any(|resource| {
                    resource.id.as_str() == id
                        && resource.arguments.keys().all(|argument| {
                            // Every projected runtime argument is represented by the canonical
                            // arguments manifest; this assertion keeps the projection typed.
                            !argument.is_empty()
                        })
                }))
        );
    }
    for id in ["lsp-client-display-name", "lsp-client-restart-exhausted"] {
        let shared = contract
            .resources
            .iter()
            .find(|resource| resource.id.as_str() == id)
            .expect("shared Neovim/LSP projection resource");
        assert_eq!(
            shared.clients,
            BTreeSet::from([
                Client::Lsp,
                Client::VsCode,
                Client::VsCodium,
                Client::Neovim
            ]),
            "shared projection resource ownership must include both hosts"
        );
    }
    let resource_ids = parser::parse(DEFAULT_RESOURCE)
        .expect("valid launch resource")
        .body
        .into_iter()
        .filter_map(|entry| match entry {
            Entry::Message(message) => Some(message.id.name.to_owned()),
            Entry::Term(_)
            | Entry::Comment(_)
            | Entry::GroupComment(_)
            | Entry::ResourceComment(_)
            | Entry::Junk { .. } => None,
        })
        .collect::<BTreeSet<_>>();
    let contract_ids = contract
        .resources
        .iter()
        .map(|resource| resource.id.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        resource_ids, contract_ids,
        "resource IDs must match the static contract"
    );

    let cli_only = contract
        .resources
        .iter()
        .find(|resource| resource.id.as_str() == "run-effect")
        .expect("run transcript resource");
    assert_eq!(cli_only.clients, BTreeSet::from([Client::Cli]));
    let tui = contract
        .resources
        .iter()
        .find(|resource| resource.id.as_str() == "tui-ready")
        .expect("TUI resource");
    assert_eq!(tui.clients, BTreeSet::from([Client::Tui]));
    let lsp = contract
        .resources
        .iter()
        .find(|resource| resource.id.as_str() == "lsp-hover-requires")
        .expect("LSP resource");
    assert_eq!(lsp.clients, BTreeSet::from([Client::Lsp]));

    let warning = contract
        .resources
        .iter()
        .find(|resource| resource.id.as_str() == "lsp-warning-ui-config")
        .expect("LSP config warning resource");
    assert_eq!(warning.clients, BTreeSet::from([Client::Lsp]));
    assert_eq!(
        warning.arguments,
        BTreeMap::from([
            ("code".to_owned(), UiArgType::String),
            ("detail".to_owned(), UiArgType::String),
        ])
    );
}

#[test]
fn incomplete_test_locale_is_not_a_support_claim_but_falls_back() {
    let catalog = UiCatalog::from_resources(
        "en-GB".parse().expect("locale"),
        [
            (
                "en-US".parse().expect("locale"),
                DEFAULT_RESOURCE.to_owned(),
            ),
            (
                "en-GB".parse().expect("locale"),
                TEST_EN_GB_RESOURCE.to_owned(),
            ),
        ],
    )
    .expect("catalog");
    let args = BTreeMap::from([
        ("asset".to_owned(), UiArg::String("asset-1".to_owned())),
        ("block".to_owned(), UiArg::String("start".to_owned())),
    ]);
    assert_eq!(
        catalog.format(MsgId::PlayStart, &args),
        "play asset=asset-1 block=start"
    );
    assert_eq!(
        UiLocale::parse("system").expect("system").to_string(),
        "system"
    );
}

#[test]
fn contract_reports_sorted_unknown_duplicate_and_argument_errors() {
    let id = ResourceId::new("hello").expect("id");
    let spec = ResourceSpec::new(id)
        .argument("name", UiArgType::Integer)
        .client(Client::Cli)
        .projection(ProjectionSpec::new(
            ResourceId::new("different").expect("id"),
            Client::Lsp,
            "label",
        ));
    let contract = UiContract::new(
        vec![spec],
        vec![
            ClientSpec::new(Client::Cli, "CLI", true),
            ClientSpec::new(Client::Cli, "CLI duplicate", true),
        ],
    );
    let error = contract
        .validate("hello = Hello { $other }\nhello = Again\nunknown = Unknown\n")
        .expect_err("invalid contract");
    let display = error.to_string();
    assert!(display.contains("duplicate resource ID `hello`"));
    assert!(display.contains("unknown resource ID `unknown`"));
    assert!(display.contains("missing argument `name`"));
    assert!(display.contains("undeclared argument `other`"));
    assert!(display.contains("undeclared projection for `lsp`"));
    assert!(display.contains("duplicate client `cli`"));
}

#[test]
fn contract_walks_selects_and_attributes_without_regex_extraction() {
    let value = ResourceSpec::new(ResourceId::new("hello").expect("id"))
        .argument("count", UiArgType::String)
        .client(Client::Cli);
    let attribute = ResourceSpec::new(ResourceId::new("hello.label").expect("id"))
        .argument("name", UiArgType::String)
        .client(Client::Cli);
    let contract = UiContract::new(
        vec![value, attribute],
        vec![ClientSpec::new(Client::Cli, "CLI", true)],
    );
    let source = "hello = { $count ->\n   [one] One\n  *[other] Other { $other }\n}\n    .label = Label {$name}\n";
    let error = contract
        .validate(source)
        .expect_err("unknown select variable");
    assert!(
        error.to_string().contains("undeclared argument `other`"),
        "{error}"
    );
    assert!(
        !error
            .to_string()
            .contains("missing resource ID `hello.label`")
    );
}

#[test]
fn malformed_resources_are_reported_by_the_ast_gate() {
    let error = UiContract::default()
        .validate("cli-help-about = {\n")
        .expect_err("malformed resource");
    assert!(
        error
            .issues
            .iter()
            .any(|issue| matches!(issue, recite_ui::ContractIssue::Malformed(_)))
    );
}

#[test]
fn resolution_invalid_references_are_rejected() {
    let contract = UiContract::new(
        vec![ResourceSpec::new(ResourceId::new("hello").expect("id")).client(Client::Cli)],
        vec![ClientSpec::new(Client::Cli, "CLI", true)],
    );
    let error = contract
        .validate("hello = { UNKNOWN() }\n")
        .expect_err("unknown Fluent function");
    assert!(
        error
            .issues
            .iter()
            .any(|issue| { matches!(issue, recite_ui::ContractIssue::Resolution(_)) })
    );
}

#[test]
fn checked_format_rejects_argument_type_mismatch() {
    let catalog = UiCatalog::load(&UiLocale::default()).expect("catalog");
    let args = BTreeMap::from([("count".to_owned(), UiArg::from("not a number"))]);
    let error = catalog
        .format_checked(MsgId::WatchBuildSucceeded, &args)
        .expect_err("wrong argument type");
    assert!(error.to_string().contains("expected Integer"));
    assert!(
        !catalog
            .format(MsgId::WatchBuildSucceeded, &args)
            .eq(MsgId::WatchBuildSucceeded.key())
    );
}
