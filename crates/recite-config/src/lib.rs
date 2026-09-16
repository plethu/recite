//! Shared local configuration and capability contracts for Recite tooling.
//!
//! This crate deliberately keeps the four configuration authorities separate:
//! invocation owns command-line overrides, project configuration owns dialogue
//! semantics, user configuration owns presentation preferences, and generated
//! data owns derived reports. User preference writes are explicit, atomic,
//! and preserve unrelated settings; loading never writes configuration.

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
    DocumentKeyError, PROJECT_MANIFEST_FILE, PROJECT_MANIFEST_FORMAT_VERSION,
    ProjectDiscoveryError, ProjectDiscoveryReport, ProjectManifest, ProjectSettings,
    ProjectSettingsError, allows_unscoped_source_path, discover_project, discover_unscoped_sources,
};
pub use user::{
    AuthorityValue, CONFIG_VERSION, ColorPolicy, ConfigAuthority, ConfigDiagnostic, ConfigError,
    ConfigFormat, ConfigProvenance, ConfigWriteError, ContrastPolicy, FieldPolicy, FieldProvenance,
    FieldResolutionError, InvocationOverrides, KeyHints, KeyHintsPolicy, Keymap, KeymapPolicy,
    LoadedUserConfig, PlayConfig, ResolvedField, ResolvedUiConfig, ResolvedUserConfig,
    ShowUnavailableChoicesPolicy, StateUpdateError, TextFileStore, TuiColorMode, TuiContrast,
    UiConfig, UiLocalePolicy, UserConfig, UserConfigEdit, UserConfigField, UserConfigStore,
    UserStateFile, WriterConfig, WriterConfirmExitPolicy, WriterPaneSide, WriterPaneSidePolicy,
    WriterReducedMotionPolicy, WriterTheme, WriterThemePolicy, WriterView, WriterViewPolicy,
    WriterZoomToPointerPolicy, load_user_config, load_user_config_from, load_user_config_path,
    resolve_field, resolve_user_config,
};

/// The user-facing locale type used by the existing UI resource contract.
pub use recite_ui::{UiLocale, UiLocaleError};

/// The existing core producer identity used by generated schema and capability
/// reports. Re-exporting it keeps this crate from inventing a second identity
/// type at the shared boundary.
pub use recite_core::{ProducerIdentity, ProducerIdentityError, ProducerIdentityPart};
