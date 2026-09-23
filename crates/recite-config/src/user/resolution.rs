//! Deterministic field ownership and precedence for user-facing settings.

mod config;
mod invocation;
mod policy;

pub use config::{ResolvedUiConfig, ResolvedUserConfig, resolve_user_config};
pub use invocation::InvocationOverrides;
pub use policy::{
    AuthorityValue, ColorPolicy, ContrastPolicy, FieldPolicy, FieldProvenance,
    FieldResolutionError, KeyHintsPolicy, KeymapPolicy, ResolvedField,
    ShowUnavailableChoicesPolicy, UiLocalePolicy, WriterConfirmExitPolicy, WriterMonochromePolicy,
    WriterPaneSidePolicy, WriterPresentationPolicy, WriterReducedMotionPolicy,
    WriterShortcutHintsPolicy, WriterShortcutsPolicy, WriterThemePolicy, WriterViewPolicy,
    WriterZoomToPointerPolicy, resolve_field,
};
