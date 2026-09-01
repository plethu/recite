use std::path::Path;

use recite_core::{DiagnosticCode, DiagnosticSeverity};

use super::{DiscoveryDiagnostic, display, structured_diagnostic};

impl DiscoveryDiagnostic {
    #[must_use]
    pub fn code(&self) -> DiagnosticCode {
        match self {
            Self::InvalidSourceRoot { .. } => super::INVALID_ROOT,
            Self::InvalidExclude { .. } => super::INVALID_EXCLUDE,
            Self::RootMissing { .. } => super::ROOT_MISSING,
            Self::RootRead { .. } => super::ROOT_READ,
            Self::RootOutsideProject { .. } => super::ROOT_OUTSIDE_PROJECT,
            Self::DuplicateRoot { .. } => super::DUPLICATE_ROOT,
            Self::OverlappingRoot { .. } => super::OVERLAPPING_ROOT,
            Self::RootNotDirectory { .. } => super::ROOT_NOT_DIRECTORY,
            Self::ReadDirectory { .. } => super::DISCOVERY_READ,
            Self::NonUtf8Path { .. } => super::NON_UTF8_PATH,
            Self::FileOutsideProject { .. } => super::FILE_OUTSIDE_PROJECT,
            Self::NonUtf8Source { .. } => super::NON_UTF8_SOURCE,
            Self::InvalidDocumentKey { .. } => super::INVALID_DOCUMENT_KEY,
        }
    }

    #[must_use]
    pub const fn is_warning(&self) -> bool {
        matches!(self, Self::OverlappingRoot { .. })
    }

    #[must_use]
    pub fn as_core_diagnostic(&self) -> recite_core::Diagnostic {
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
            Self::DuplicateRoot { path, .. } => (
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
                    "project source symlink {} resolves outside its configured source root or project to {}",
                    display(path),
                    display(target)
                ),
            ),
            Self::NonUtf8Source { path } => (
                path,
                format!("project source is not valid UTF-8: {}", display(path)),
            ),
            Self::InvalidDocumentKey { path, reason } => (
                path,
                format!("project source has an invalid document key: {reason}"),
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
