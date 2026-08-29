use std::path::{Path, PathBuf};

use recite_core::{Diagnostic, DiagnosticCode, SourcePosition, SourceSpan};

use crate::path::{CONFIG_ENVIRONMENT_VARIABLE, PathResolutionError};

const EMPTY_OVERRIDE: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG001");
const RELATIVE_OVERRIDE: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG002");
const MISSING_EXPLICIT: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG003");
const READ_FAILURE: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG004");
const MALFORMED_CONFIG: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG005");
const UNSUPPORTED_VERSION: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG006");
const INVALID_LOCALE: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG007");

/// A structured identity for one configuration failure.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ConfigDiagnostic {
    /// Empty explicit override.
    EmptyExplicitOverride,
    /// Relative explicit override.
    RelativeExplicitOverride,
    /// Missing explicit override target.
    MissingExplicitOverride,
    /// Read failure.
    ReadFailure,
    /// TOML or owned-field failure.
    Malformed,
    /// Unsupported config format version.
    UnsupportedVersion,
    /// Invalid UI locale.
    InvalidLocale,
}

impl ConfigDiagnostic {
    /// Stable core diagnostic identity for this failure.
    #[must_use]
    pub const fn code(self) -> DiagnosticCode {
        match self {
            Self::EmptyExplicitOverride => EMPTY_OVERRIDE,
            Self::RelativeExplicitOverride => RELATIVE_OVERRIDE,
            Self::MissingExplicitOverride => MISSING_EXPLICIT,
            Self::ReadFailure => READ_FAILURE,
            Self::Malformed => MALFORMED_CONFIG,
            Self::UnsupportedVersion => UNSUPPORTED_VERSION,
            Self::InvalidLocale => INVALID_LOCALE,
        }
    }
}

/// Typed failure from path resolution or user configuration loading.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// Explicit path interpretation failed.
    #[error(transparent)]
    Path(#[from] PathResolutionError),
    /// Explicit path does not exist.
    #[error("explicit {CONFIG_ENVIRONMENT_VARIABLE} path does not exist: {path}")]
    MissingExplicit { path: PathBuf },
    /// A selected config could not be read.
    #[error("could not read user config {path}: {message}")]
    Read { path: PathBuf, message: String },
    /// TOML or a strict owned field was malformed.
    #[error("could not parse user config {path}: {message}")]
    Malformed { path: PathBuf, message: String },
    /// A future or otherwise unsupported config version was supplied.
    #[error("user config {path} uses unsupported version {found}; expected {expected}")]
    UnsupportedVersion {
        /// Offending path.
        path: PathBuf,
        /// Supplied version.
        found: u32,
        /// Supported version.
        expected: u32,
    },
    /// A UI locale is not a valid BCP-47 value or `system`.
    #[error("user config {path} has invalid UI locale {locale:?}")]
    InvalidLocale { path: PathBuf, locale: String },
}

impl ConfigError {
    /// Returns the stable structured diagnostic identity for this failure.
    #[must_use]
    pub fn diagnostic(&self) -> ConfigDiagnostic {
        match self {
            Self::Path(PathResolutionError::EmptyExplicitOverride) => {
                ConfigDiagnostic::EmptyExplicitOverride
            }
            Self::Path(PathResolutionError::RelativeExplicitOverride { .. }) => {
                ConfigDiagnostic::RelativeExplicitOverride
            }
            Self::MissingExplicit { .. } => ConfigDiagnostic::MissingExplicitOverride,
            Self::Read { .. } => ConfigDiagnostic::ReadFailure,
            Self::Malformed { .. } => ConfigDiagnostic::Malformed,
            Self::UnsupportedVersion { .. } => ConfigDiagnostic::UnsupportedVersion,
            Self::InvalidLocale { .. } => ConfigDiagnostic::InvalidLocale,
        }
    }

    /// Returns a core diagnostic with a deterministic point span at the config
    /// path. The structured code remains available through [`Self::diagnostic`].
    #[must_use]
    pub fn as_core_diagnostic(&self) -> Diagnostic {
        Diagnostic::error(
            self.diagnostic().code(),
            self.to_string(),
            config_span(self.path()),
        )
    }

    fn path(&self) -> &Path {
        match self {
            Self::Path(_) => Path::new(CONFIG_ENVIRONMENT_VARIABLE),
            Self::MissingExplicit { path }
            | Self::Read { path, .. }
            | Self::Malformed { path, .. }
            | Self::UnsupportedVersion { path, .. }
            | Self::InvalidLocale { path, .. } => path,
        }
    }
}

fn config_span(path: &Path) -> SourceSpan {
    let position = match SourcePosition::new(1, 1) {
        Ok(position) => position,
        Err(error) => unreachable!("one-based config diagnostic span: {error}"),
    };
    SourceSpan::point(path.to_string_lossy().into_owned(), position)
}
