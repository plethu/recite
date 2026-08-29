//! Shared local configuration and capability contracts for Recite tooling.
//!
//! This crate deliberately keeps the four configuration authorities separate:
//! invocation owns command-line overrides, project configuration owns dialogue
//! semantics, user configuration owns presentation preferences, and generated
//! data owns derived reports. It does not merge those authorities or write any
//! of them. The user configuration loader here is read-only and local-first.

mod capabilities;
mod path;
mod project;
mod user;

pub use capabilities::{
    CAPABILITY_REPORT_VERSION, Capability, CapabilityId, CapabilityName, CapabilityNameError,
    CapabilityReport, CapabilityReportError, CapabilityStatus,
};
pub use path::{
    CONFIG_ENVIRONMENT_VARIABLE, ConfigPathSource, PathResolutionError, Platform, PlatformRoots,
    ResolvedConfigPath, production_config_path, resolve_config_path,
};
pub use project::{
    Coverage, DiscoveredDocument, DiscoveredRoot, DiscoveryDiagnostic, DocumentKey,
    PROJECT_MANIFEST_FILE, PROJECT_MANIFEST_FORMAT_VERSION, ProjectDiscoveryError,
    ProjectDiscoveryReport, ProjectManifest, discover_project, discover_unscoped_sources,
};
pub use user::{
    AuthorityValue, CONFIG_VERSION, ColorPolicy, ConfigAuthority, ConfigDiagnostic, ConfigError,
    ConfigFormat, ConfigProvenance, ContrastPolicy, FieldPolicy, FieldProvenance,
    FieldResolutionError, InvocationOverrides, KeyHints, KeyHintsPolicy, Keymap, KeymapPolicy,
    LoadedUserConfig, PlayConfig, ResolvedField, ResolvedUiConfig, ResolvedUserConfig,
    ShowUnavailableChoicesPolicy, TuiColorMode, TuiContrast, UiConfig, UiLocalePolicy, UserConfig,
    UserConfigField, load_user_config, load_user_config_from, load_user_config_path, resolve_field,
    resolve_user_config,
};

/// The user-facing locale type used by the existing UI resource contract.
pub use recite_ui::{UiLocale, UiLocaleError};

/// The existing core producer identity used by generated schema and capability
/// reports. Re-exporting it keeps this crate from inventing a second identity
/// type at the shared boundary.
pub use recite_core::{ProducerIdentity, ProducerIdentityError, ProducerIdentityPart};
