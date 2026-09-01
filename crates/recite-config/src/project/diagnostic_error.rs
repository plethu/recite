use std::path::Path;

use recite_core::DiagnosticCode;

use super::ProjectDiscoveryError;

impl ProjectDiscoveryError {
    /// The manifest path involved in this failure, when a manifest was found.
    #[must_use]
    pub fn manifest_path(&self) -> Option<&Path> {
        match self {
            Self::NotFound { .. } => None,
            Self::Read { path, .. }
            | Self::NonUtf8 { path }
            | Self::Malformed { path, .. }
            | Self::MissingFormatVersion { path, .. }
            | Self::UnsupportedFormatVersion { path, .. }
            | Self::InvalidSourceRoot { path, .. }
            | Self::InvalidExclude { path, .. } => Some(path),
            Self::DuplicateRoot { manifest, .. } => Some(manifest),
        }
    }

    /// Diagnostics suitable for an editor after a failed discovery attempt.
    /// Parser diagnostics retain their source-backed spans and stable codes.
    #[must_use]
    pub fn diagnostics(&self) -> Vec<recite_core::Diagnostic> {
        match self {
            Self::Malformed { diagnostics, .. } if !diagnostics.is_empty() => diagnostics.clone(),
            _ => vec![self.as_core_diagnostic()],
        }
    }

    #[must_use]
    pub fn diagnostic(&self) -> DiagnosticCode {
        match self {
            Self::NotFound { .. } => super::MISSING_MANIFEST,
            Self::Read { .. } | Self::NonUtf8 { .. } => super::MANIFEST_READ,
            Self::Malformed { .. } => super::MANIFEST_MALFORMED,
            Self::MissingFormatVersion { .. } | Self::UnsupportedFormatVersion { .. } => {
                super::MANIFEST_VERSION
            }
            Self::InvalidSourceRoot { .. } => super::INVALID_ROOT,
            Self::InvalidExclude { .. } => super::INVALID_EXCLUDE,
            Self::DuplicateRoot { .. } => super::DUPLICATE_ROOT,
        }
    }

    #[must_use]
    pub fn as_core_diagnostic(&self) -> recite_core::Diagnostic {
        let path = match self {
            Self::NotFound { start } => start,
            Self::Read { path, .. }
            | Self::NonUtf8 { path }
            | Self::Malformed { path, .. }
            | Self::MissingFormatVersion { path, .. }
            | Self::UnsupportedFormatVersion { path, .. }
            | Self::InvalidSourceRoot { path, .. }
            | Self::InvalidExclude { path, .. }
            | Self::DuplicateRoot { path, .. } => path,
        };
        super::structured_diagnostic(
            self.diagnostic(),
            recite_core::DiagnosticSeverity::Error,
            self.to_string(),
            path,
        )
    }
}
