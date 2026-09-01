use std::path::{Path, PathBuf};

use recite_core::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticCode, DiagnosticSeverity, SourcePosition,
    SourceSpan,
};

use super::manifest::PROJECT_MANIFEST_FILE;

#[path = "diagnostic_error.rs"]
mod diagnostic_error;
#[path = "diagnostic_render.rs"]
mod diagnostic_render;

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
pub(super) const INVALID_DOCUMENT_KEY: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_CONFIG117");

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
    InvalidDocumentKey { path: PathBuf, reason: String },
}

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
    DuplicateRoot { path: PathBuf, manifest: PathBuf },
}

fn structured_diagnostic(
    code: DiagnosticCode,
    severity: DiagnosticSeverity,
    message: String,
    path: &Path,
) -> Diagnostic {
    let diagnostic = Diagnostic::new(code.clone(), severity, message.clone(), point_span(path));
    let Some(contract) = recite_core::config_contract_for(&code) else {
        return diagnostic;
    };
    let Ok(presentation) =
        contract.presentation([("detail", DiagnosticArgumentValue::String(message))])
    else {
        return diagnostic;
    };
    diagnostic.with_presentation(presentation)
}

fn point_span(path: &Path) -> SourceSpan {
    let position = match SourcePosition::new(1, 1) {
        Ok(position) => position,
        Err(error) => unreachable!("one-based project diagnostic span: {error}"),
    };
    SourceSpan::point(path.to_string_lossy().into_owned(), position)
}

fn display(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
