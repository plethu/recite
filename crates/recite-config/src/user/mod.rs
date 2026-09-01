mod diagnostics;
mod load;
mod model;
mod resolution;

pub use diagnostics::{ConfigDiagnostic, ConfigError};
pub use load::{load_user_config, load_user_config_from, load_user_config_path};
pub use model::{
    CONFIG_VERSION, ConfigAuthority, ConfigFormat, ConfigProvenance, KeyHints, Keymap,
    LoadedUserConfig, PlayConfig, TuiColorMode, TuiContrast, UiConfig, UserConfig, UserConfigField,
};
pub use resolution::{
    AuthorityValue, ColorPolicy, ContrastPolicy, FieldPolicy, FieldProvenance,
    FieldResolutionError, InvocationOverrides, KeyHintsPolicy, KeymapPolicy, ResolvedField,
    ResolvedUiConfig, ResolvedUserConfig, ShowUnavailableChoicesPolicy, UiLocalePolicy,
    resolve_field, resolve_user_config,
};
