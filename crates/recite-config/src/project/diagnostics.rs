use std::path::{Path, PathBuf};

use recite_core::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentationId,
    DiagnosticSeverity, SourcePosition, SourceSpan,
};

use super::manifest::PROJECT_MANIFEST_FILE;

pub(super) const MISSING_MANIFEST: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG101");
pub(super) const MANIFEST_READ: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG102");
pub(super) const MANIFEST_MALFORMED: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_CONFIG103");
pub(super) const MANIFEST_VERSION: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG104");
pub(super) const INVALID_ROOT: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG105");
pub(super) const INVALID_EXCLUDE: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG106");
pub(super) const ROOT_MISSING: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG107");
pub(super) const ROOT_READ: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG108");
pub(super) const ROOT_OUTSIDE_PROJECT: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_CONFIG109");
pub(super) const DUPLICATE_ROOT: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG110");
pub(super) const OVERLAPPING_ROOT: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG111");
pub(super) const DISCOVERY_READ: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG112");
pub(super) const NON_UTF8_PATH: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG113");
pub(super) const FILE_OUTSIDE_PROJECT: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_CONFIG114");
pub(super) const NON_UTF8_SOURCE: DiagnosticCode = DiagnosticCode::new_static("RECITE_CONFIG115");
pub(super) const ROOT_NOT_DIRECTORY: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_CONFIG116");

/// A typed, deterministic discovery diagnostic. Warnings remain in the report
/// so callers can present overlap policy without reconstructing it from text.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DiscoveryDiagnostic {
    InvalidSourceRoot { root: String, reason: String },
    InvalidExclude { pattern: String, reason: String },
    RootMissing { path: PathBuf },
    RootRead { path: PathBuf, message: String },
    RootOutsideProject { path: PathBuf },
    DuplicateRoot { path: PathBuf },
    OverlappingRoot { path: PathBuf, owner: PathBuf },
    RootNotDirectory { path: PathBuf },
    ReadDirectory { path: PathBuf, message: String },
    NonUtf8Path { path: PathBuf },
    FileOutsideProject { path: PathBuf, target: PathBuf },
    NonUtf8Source { path: PathBuf },
}

impl DiscoveryDiagnostic {
    #[must_use]
    pub fn code(&self) -> DiagnosticCode {
        match self {
            Self::InvalidSourceRoot { .. } => INVALID_ROOT,
            Self::InvalidExclude { .. } => INVALID_EXCLUDE,
            Self::RootMissing { .. } => ROOT_MISSING,
            Self::RootRead { .. } => ROOT_READ,
            Self::RootOutsideProject { .. } => ROOT_OUTSIDE_PROJECT,
            Self::DuplicateRoot { .. } => DUPLICATE_ROOT,
            Self::OverlappingRoot { .. } => OVERLAPPING_ROOT,
            Self::RootNotDirectory { .. } => ROOT_NOT_DIRECTORY,
            Self::ReadDirectory { .. } => DISCOVERY_READ,
            Self::NonUtf8Path { .. } => NON_UTF8_PATH,
            Self::FileOutsideProject { .. } => FILE_OUTSIDE_PROJECT,
            Self::NonUtf8Source { .. } => NON_UTF8_SOURCE,
        }
    }

    #[must_use]
    pub const fn is_warning(&self) -> bool {
        matches!(self, Self::OverlappingRoot { .. })
    }

    #[must_use]
    pub fn as_core_diagnostic(&self) -> Diagnostic {
        let (path, message) = self.path_and_message();
        let severity = if self.is_warning() {
            DiagnosticSeverity::Warning
        } else {
            DiagnosticSeverity::Error
        };
        structured_diagnostic(self.code(), severity, message, path)
    }

    fn path_and_message(&self) -> (&Path, String) {
        match self {
            Self::InvalidSourceRoot { root, reason } => (
                Path::new(root),
                format!("invalid project source root {root:?}: {reason}"),
            ),
            Self::InvalidExclude { pattern, reason } => (
                Path::new(pattern),
                format!("invalid project exclude {pattern:?}: {reason}"),
            ),
            Self::RootMissing { path } => (
                path,
                format!("project source root does not exist: {}", display(path)),
            ),
            Self::RootRead { path, message } => (
                path,
                format!(
                    "could not read project source root {}: {message}",
                    display(path)
                ),
            ),
            Self::RootOutsideProject { path } => (
                path,
                format!("project source root escapes the project: {}", display(path)),
            ),
            Self::DuplicateRoot { path } => (
                path,
                format!(
                    "project source root is listed more than once: {}",
                    display(path)
                ),
            ),
            Self::OverlappingRoot { path, owner } => (
                path,
                format!(
                    "project source root {} overlaps earlier root {}; earlier root owns shared documents",
                    display(path),
                    display(owner)
                ),
            ),
            Self::RootNotDirectory { path } => (
                path,
                format!("project source root is not a directory: {}", display(path)),
            ),
            Self::ReadDirectory { path, message } => (
                path,
                format!(
                    "could not read project source directory {}: {message}",
                    display(path)
                ),
            ),
            Self::NonUtf8Path { path } => (
                path,
                format!(
                    "project discovery cannot represent a non-UTF-8 path near {}",
                    display(path)
                ),
            ),
            Self::FileOutsideProject { path, target } => (
                path,
                format!(
                    "project source symlink {} resolves outside the project to {}",
                    display(path),
                    display(target)
                ),
            ),
            Self::NonUtf8Source { path } => (
                path,
                format!("project source is not valid UTF-8: {}", display(path)),
            ),
        }
    }
}

impl std::fmt::Display for DiscoveryDiagnostic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.path_and_message().1)
    }
}

impl std::error::Error for DiscoveryDiagnostic {}

/// Failure that prevents a project index from being constructed.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProjectDiscoveryError {
    #[error("could not find {PROJECT_MANIFEST_FILE} from {start}")]
    NotFound { start: PathBuf },
    #[error("could not read project manifest {path}: {message}")]
    Read { path: PathBuf, message: String },
    #[error("project manifest {path} is not valid UTF-8")]
    NonUtf8 { path: PathBuf },
    #[error("project manifest {path} is malformed: {detail}")]
    Malformed {
        path: PathBuf,
        detail: String,
        diagnostics: Vec<Diagnostic>,
    },
    #[error("project manifest {path} must declare format_version = {expected}")]
    MissingFormatVersion { path: PathBuf, expected: u32 },
    #[error("project manifest {path} uses unsupported format_version {found}; expected {expected}")]
    UnsupportedFormatVersion {
        path: PathBuf,
        found: u32,
        expected: u32,
    },
    #[error("project manifest {path} has invalid source root {root:?}: {reason}")]
    InvalidSourceRoot {
        path: PathBuf,
        root: String,
        reason: String,
    },
    #[error("project manifest {path} has invalid exclude {pattern:?}: {reason}")]
    InvalidExclude {
        path: PathBuf,
        pattern: String,
        reason: String,
    },
    #[error("project source roots contain duplicate canonical path {path}")]
    DuplicateRoot { path: PathBuf },
}

impl ProjectDiscoveryError {
    #[must_use]
    pub fn diagnostic(&self) -> DiagnosticCode {
        match self {
            Self::NotFound { .. } => MISSING_MANIFEST,
            Self::Read { .. } | Self::NonUtf8 { .. } => MANIFEST_READ,
            Self::Malformed { .. } => MANIFEST_MALFORMED,
            Self::MissingFormatVersion { .. } | Self::UnsupportedFormatVersion { .. } => {
                MANIFEST_VERSION
            }
            Self::InvalidSourceRoot { .. } => INVALID_ROOT,
            Self::InvalidExclude { .. } => INVALID_EXCLUDE,
            Self::DuplicateRoot { .. } => DUPLICATE_ROOT,
        }
    }

    #[must_use]
    pub fn as_core_diagnostic(&self) -> Diagnostic {
        let path = match self {
            Self::NotFound { start } => start,
            Self::Read { path, .. }
            | Self::NonUtf8 { path }
            | Self::Malformed { path, .. }
            | Self::MissingFormatVersion { path, .. }
            | Self::UnsupportedFormatVersion { path, .. }
            | Self::InvalidSourceRoot { path, .. }
            | Self::InvalidExclude { path, .. }
            | Self::DuplicateRoot { path } => path,
        };
        structured_diagnostic(
            self.diagnostic(),
            DiagnosticSeverity::Error,
            self.to_string(),
            path,
        )
    }
}

#[allow(
    clippy::expect_used,
    reason = "the config diagnostic inventory is defined alongside these producers"
)]
fn structured_diagnostic(
    code: DiagnosticCode,
    severity: DiagnosticSeverity,
    message: String,
    path: &Path,
) -> Diagnostic {
    let presentation_id = match code.as_str() {
        "RECITE_CONFIG101" => DiagnosticPresentationId::new_static("diagnostic-config-101"),
        "RECITE_CONFIG102" => DiagnosticPresentationId::new_static("diagnostic-config-102"),
        "RECITE_CONFIG103" => DiagnosticPresentationId::new_static("diagnostic-config-103"),
        "RECITE_CONFIG104" => DiagnosticPresentationId::new_static("diagnostic-config-104"),
        "RECITE_CONFIG105" => DiagnosticPresentationId::new_static("diagnostic-config-105"),
        "RECITE_CONFIG106" => DiagnosticPresentationId::new_static("diagnostic-config-106"),
        "RECITE_CONFIG107" => DiagnosticPresentationId::new_static("diagnostic-config-107"),
        "RECITE_CONFIG108" => DiagnosticPresentationId::new_static("diagnostic-config-108"),
        "RECITE_CONFIG109" => DiagnosticPresentationId::new_static("diagnostic-config-109"),
        "RECITE_CONFIG110" => DiagnosticPresentationId::new_static("diagnostic-config-110"),
        "RECITE_CONFIG111" => DiagnosticPresentationId::new_static("diagnostic-config-111"),
        "RECITE_CONFIG112" => DiagnosticPresentationId::new_static("diagnostic-config-112"),
        "RECITE_CONFIG113" => DiagnosticPresentationId::new_static("diagnostic-config-113"),
        "RECITE_CONFIG114" => DiagnosticPresentationId::new_static("diagnostic-config-114"),
        "RECITE_CONFIG115" => DiagnosticPresentationId::new_static("diagnostic-config-115"),
        "RECITE_CONFIG116" => DiagnosticPresentationId::new_static("diagnostic-config-116"),
        _ => panic!("unknown config diagnostic code: {code}"),
    };
    let presentation = recite_core::presentation_for(
        &code,
        &presentation_id,
        [("detail", DiagnosticArgumentValue::String(message.clone()))],
    )
    .expect("config diagnostic presentation contract must exist");
    Diagnostic::new(code, severity, message, point_span(path)).with_presentation(presentation)
}

fn point_span(path: &Path) -> SourceSpan {
    #[allow(clippy::expect_used)]
    let position = SourcePosition::new(1, 1).expect("one-based project diagnostic span");
    SourceSpan::point(path.to_string_lossy().into_owned(), position)
}

fn display(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
