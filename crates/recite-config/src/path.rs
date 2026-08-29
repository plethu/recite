use std::{
    env, fmt,
    path::{Path, PathBuf},
};

/// Environment variable selecting an explicit user configuration file.
pub const CONFIG_ENVIRONMENT_VARIABLE: &str = "RECITE_CONFIG";

/// Operating systems with a first-class Recite desktop configuration location.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Platform {
    /// Linux and other systems following the XDG configuration convention.
    Linux,
    /// macOS Application Support.
    MacOs,
    /// Windows roaming application data.
    Windows,
}

/// Roots used by the pure platform path calculator.
///
/// Production code obtains these roots through [`dirs`] in
/// [`production_config_path`]. Tests can instead provide synthetic absolute
/// paths without mutating process-global environment variables.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PlatformRoots {
    /// `$XDG_CONFIG_HOME`, when set on Linux.
    pub xdg_config_home: Option<PathBuf>,
    /// `$HOME`, used for Linux's `~/.config` fallback.
    pub home: Option<PathBuf>,
    /// macOS `Application Support` directory.
    pub application_support: Option<PathBuf>,
    /// Windows roaming `AppData` directory.
    pub roaming_app_data: Option<PathBuf>,
}

impl PlatformRoots {
    /// Creates empty roots. An absent default root means default user settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            xdg_config_home: None,
            home: None,
            application_support: None,
            roaming_app_data: None,
        }
    }

    /// Supplies a Linux XDG configuration root.
    #[must_use]
    pub fn with_xdg_config_home(mut self, root: impl Into<PathBuf>) -> Self {
        self.xdg_config_home = Some(root.into());
        self
    }

    /// Supplies a home directory for the Linux XDG fallback.
    #[must_use]
    pub fn with_home(mut self, root: impl Into<PathBuf>) -> Self {
        self.home = Some(root.into());
        self
    }

    /// Supplies a macOS Application Support root.
    #[must_use]
    pub fn with_application_support(mut self, root: impl Into<PathBuf>) -> Self {
        self.application_support = Some(root.into());
        self
    }

    /// Supplies a Windows roaming AppData root.
    #[must_use]
    pub fn with_roaming_app_data(mut self, root: impl Into<PathBuf>) -> Self {
        self.roaming_app_data = Some(root.into());
        self
    }
}

/// Why a resolved path was selected.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConfigPathSource {
    /// The caller explicitly selected `$RECITE_CONFIG`.
    ExplicitOverride,
    /// The platform strategy selected the path.
    PlatformDefault,
}

/// A configuration path with its authority/provenance preserved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedConfigPath {
    path: PathBuf,
    source: ConfigPathSource,
}

impl ResolvedConfigPath {
    fn new(path: PathBuf, source: ConfigPathSource) -> Self {
        Self { path, source }
    }

    /// Returns the selected file path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the authority that selected this path.
    #[must_use]
    pub const fn source(&self) -> ConfigPathSource {
        self.source
    }

    /// Whether this path came from an explicit environment override.
    #[must_use]
    pub const fn is_explicit(&self) -> bool {
        matches!(self.source, ConfigPathSource::ExplicitOverride)
    }
}

/// Failure while interpreting the explicit path or platform inputs.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum PathResolutionError {
    /// `$RECITE_CONFIG` was present but empty.
    #[error("{CONFIG_ENVIRONMENT_VARIABLE} is set but empty")]
    EmptyExplicitOverride,
    /// `$RECITE_CONFIG` must name an absolute path.
    #[error("{CONFIG_ENVIRONMENT_VARIABLE} must be an absolute path: {path}")]
    RelativeExplicitOverride { path: PathBuf },
}

/// Calculates the user config path without consulting the process environment.
///
/// An explicit path is an authority boundary: empty and relative values fail.
/// Missing platform roots return `Ok(None)`, allowing callers to use defaults.
/// File existence and readability are checked by the user loader rather than
/// this pure path calculation.
pub fn resolve_config_path(
    platform: Platform,
    roots: &PlatformRoots,
    explicit_override: Option<&Path>,
) -> Result<Option<ResolvedConfigPath>, PathResolutionError> {
    if let Some(path) = explicit_override {
        if path.as_os_str().is_empty() {
            return Err(PathResolutionError::EmptyExplicitOverride);
        }
        if !path.is_absolute() {
            return Err(PathResolutionError::RelativeExplicitOverride {
                path: path.to_path_buf(),
            });
        }
        return Ok(Some(ResolvedConfigPath::new(
            path.to_path_buf(),
            ConfigPathSource::ExplicitOverride,
        )));
    }

    let path = match platform {
        Platform::Linux => {
            if let Some(root) = roots.xdg_config_home.as_deref() {
                root.join("recite").join("config.toml")
            } else if let Some(home) = roots.home.as_deref() {
                home.join(".config").join("recite").join("config.toml")
            } else {
                return Ok(None);
            }
        }
        Platform::MacOs => {
            let Some(root) = roots.application_support.as_deref() else {
                return Ok(None);
            };
            root.join("Recite").join("config.toml")
        }
        Platform::Windows => {
            let Some(root) = roots.roaming_app_data.as_deref() else {
                return Ok(None);
            };
            root.join("Recite").join("config.toml")
        }
    };

    Ok(Some(ResolvedConfigPath::new(
        path,
        ConfigPathSource::PlatformDefault,
    )))
}

/// Resolves the production path from the environment and the current platform.
///
/// This is the sole adapter around [`dirs`]. The pure calculator above remains
/// the test seam, so contract tests never mutate process-global environment.
pub fn production_config_path() -> Result<Option<ResolvedConfigPath>, PathResolutionError> {
    let explicit = env::var_os(CONFIG_ENVIRONMENT_VARIABLE).map(PathBuf::from);
    let default_root = dirs::config_dir();
    let mut roots = PlatformRoots::new();
    match current_platform() {
        Platform::Linux => roots.xdg_config_home = default_root,
        Platform::MacOs => roots.application_support = default_root,
        Platform::Windows => roots.roaming_app_data = default_root,
    }
    resolve_config_path(current_platform(), &roots, explicit.as_deref())
}

#[cfg(target_os = "macos")]
const fn current_platform() -> Platform {
    Platform::MacOs
}

#[cfg(target_os = "windows")]
const fn current_platform() -> Platform {
    Platform::Windows
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const fn current_platform() -> Platform {
    Platform::Linux
}

impl fmt::Display for ConfigPathSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExplicitOverride => formatter.write_str("explicit override"),
            Self::PlatformDefault => formatter.write_str("platform default"),
        }
    }
}
