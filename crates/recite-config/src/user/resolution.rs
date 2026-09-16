//! Deterministic field ownership and precedence for user-facing settings.

mod config;
mod policy;

pub use config::{InvocationOverrides, ResolvedUiConfig, ResolvedUserConfig, resolve_user_config};
pub use policy::{
    AuthorityValue, ColorPolicy, ContrastPolicy, FieldPolicy, FieldProvenance,
    FieldResolutionError, KeyHintsPolicy, KeymapPolicy, ResolvedField,
    ShowUnavailableChoicesPolicy, UiLocalePolicy, WriterConfirmExitPolicy,
    WriterReducedMotionPolicy, WriterThemePolicy, WriterViewPolicy, WriterZoomToPointerPolicy,
    resolve_field,
};
