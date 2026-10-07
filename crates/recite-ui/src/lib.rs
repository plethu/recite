//! The shared, host-neutral Fluent boundary for Recite-owned UI text.
//!
//! Dialogue text and semantic diagnostics deliberately do not depend on this
//! crate. Clients provide a [`UiCatalog`] at their presentation edge, while
//! machine-facing values remain locale-neutral.
//!
//! The human-authored en-US resources and typed [`MsgId`] registry own UI text;
//! `resources/arguments.toml` owns its arguments. Generated host projections
//! retain those IDs and are read-only. Publishing another UI locale requires
//! human authorship and review; en-GB is only a test fixture.
//!
//! Catalog lookup tries the requested BCP-47 locale, its language, then en-US.
//! Invalid non-default resources are rejected atomically; an invalid default
//! resource is fatal. Semantic producers provide structured presentations and
//! clients render them here. Protocol values and dialogue localisation keep
//! their own boundaries. `DiagnosticRecord` v1's legacy message fallback is a
//! closed compatibility surface: removing it requires a versioned migration.

mod args;
mod catalog;
mod contract;
mod inventory;
mod locale;

pub use args::{UiArg, UiArgType, UiArgs};
pub use catalog::{CatalogError, RenderedDiagnostic, RenderedRelatedDiagnostic, UiCatalog};
pub use contract::{
    Client, ClientSpec, ContractIssue, ProjectionSpec, ResourceSpec, UiContract, UiContractError,
    all_resource_specs,
};
pub use inventory::{ALL_MESSAGE_IDS, MESSAGE_COUNT, MsgId, ResourceId, ResourceIdError};
pub use locale::{DEFAULT_LOCALE, UiLocale, UiLocaleError, fallback_chain};

/// The human-authored launch resource. This is the only locale bundled as a
/// support claim; other resources are fixtures until they pass human review.
pub const DEFAULT_RESOURCE: &str = concat!(
    include_str!("../resources/en-US.ftl"),
    "\n",
    include_str!("../resources/diagnostics.ftl")
);

/// The sole non-localised compatibility adapter for legacy diagnostic
/// producers that still provide only deterministic English message text.
pub const LEGACY_DIAGNOSTIC_RESOURCE: &str = "diagnostic-legacy-message";

/// A deliberately incomplete fixture for fallback and completeness tests.
/// It is not loaded by [`UiCatalog::load`] and is not a supported locale.
pub const TEST_EN_GB_RESOURCE: &str = include_str!("../resources/test/en-GB.ftl");

/// Every client that may project Recite-owned UI text.
pub const CLIENT_INVENTORY: &[ClientSpec] = &[
    ClientSpec::new(Client::Cli, "CLI", true),
    ClientSpec::new(Client::Tui, "TUI", true),
    ClientSpec::new(Client::Lsp, "LSP", true),
    ClientSpec::new(Client::VsCode, "VS Code", false),
    ClientSpec::new(Client::VsCodium, "VSCodium", false),
    ClientSpec::new(Client::Neovim, "Neovim", true),
    ClientSpec::new(Client::Zed, "Zed", false),
    ClientSpec::new(Client::NativeGui, "native GUI", false),
];

/// Validate the checked-in launch resource against the complete inventory.
pub fn validate_default_resource() -> Result<(), UiContractError> {
    UiContract::default().validate(DEFAULT_RESOURCE)
}
