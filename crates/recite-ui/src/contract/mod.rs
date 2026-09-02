mod ast;
mod diagnostics;
mod issue;
mod ownership;
mod projections;
mod validate;

use std::collections::{BTreeMap, BTreeSet};

use crate::{MsgId, ResourceId, UiArgType};

use self::ast::argument_contract;

pub use issue::{ContractIssue, UiContractError};

/// A first-party or future client consuming Recite-owned UI resources.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Client {
    Cli,
    Tui,
    Lsp,
    VsCode,
    VsCodium,
    Neovim,
    Zed,
    NativeGui,
}

impl Client {
    pub const ALL: &'static [Self] = &[
        Self::Cli,
        Self::Tui,
        Self::Lsp,
        Self::VsCode,
        Self::VsCodium,
        Self::Neovim,
        Self::Zed,
        Self::NativeGui,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::Cli => "cli",
            Self::Tui => "tui",
            Self::Lsp => "lsp",
            Self::VsCode => "vs-code",
            Self::VsCodium => "vscodium",
            Self::Neovim => "neovim",
            Self::Zed => "zed",
            Self::NativeGui => "native-gui",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClientSpec {
    pub client: Client,
    pub name: &'static str,
    pub shipped: bool,
}

impl ClientSpec {
    pub const fn new(client: Client, name: &'static str, shipped: bool) -> Self {
        Self {
            client,
            name,
            shipped,
        }
    }
}

/// A read-only host projection must name the source resource it projects.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProjectionSpec {
    pub source: ResourceId,
    pub client: Client,
    pub field: String,
}

impl ProjectionSpec {
    pub fn new(source: ResourceId, client: Client, field: impl Into<String>) -> Self {
        Self {
            source,
            client,
            field: field.into(),
        }
    }
}

/// Contract metadata for one resource ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceSpec {
    pub id: ResourceId,
    pub arguments: BTreeMap<String, UiArgType>,
    pub clients: BTreeSet<Client>,
    pub projections: BTreeSet<ProjectionSpec>,
    duplicate_arguments: BTreeSet<String>,
}

impl ResourceSpec {
    pub fn new(id: ResourceId) -> Self {
        Self {
            id,
            arguments: BTreeMap::new(),
            clients: BTreeSet::new(),
            projections: BTreeSet::new(),
            duplicate_arguments: BTreeSet::new(),
        }
    }

    pub fn argument(mut self, name: impl Into<String>, kind: UiArgType) -> Self {
        let name = name.into();
        if self.arguments.insert(name.clone(), kind).is_some() {
            self.duplicate_arguments.insert(name);
        }
        self
    }

    pub fn client(mut self, client: Client) -> Self {
        self.clients.insert(client);
        self
    }

    pub fn projection(mut self, projection: ProjectionSpec) -> Self {
        self.projections.insert(projection);
        self
    }
}

/// The complete shared inventory and client ownership declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UiContract {
    pub resources: Vec<ResourceSpec>,
    pub clients: Vec<ClientSpec>,
}

/// Return the generated typed resource registry in deterministic ID order.
pub fn all_resource_specs() -> Vec<ResourceSpec> {
    UiContract::default().resources
}

impl Default for UiContract {
    fn default() -> Self {
        let declared = argument_contract();
        let resources = MsgId::ALL
            .iter()
            .map(|id| {
                let mut spec = ResourceSpec::new(id.resource_id());
                if let Some(arguments) = declared.get(id.key()) {
                    for (name, kind) in arguments {
                        spec = spec.argument(name.clone(), *kind);
                    }
                }
                // Ownership is an explicit per-ID registry, not inferred from
                // naming prefixes. Future clients remain conformance entries.
                for client in ownership::clients(*id) {
                    spec = spec.client(*client);
                }
                for projection in projections::for_message(*id) {
                    spec = spec.projection(ProjectionSpec::new(
                        id.resource_id(),
                        projection.client,
                        projection.field,
                    ));
                }
                spec
            })
            .chain(diagnostics::resource_specs())
            .chain(std::iter::once(diagnostics::legacy_resource_spec()))
            .collect();
        Self {
            resources,
            clients: crate::CLIENT_INVENTORY.to_vec(),
        }
    }
}
