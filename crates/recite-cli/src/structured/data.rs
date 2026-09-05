use std::fs;
use std::path::Path;

use recite_compiler::PotEntry;
use recite_core::{Diagnostic, DiagnosticRecord};
use serde::Serialize;

use super::errors::StructuredError;
use crate::error::CliError;
use crate::runtime_fixture::TraceDocument;
use crate::schema_inspection::{MachinePathProjection, machine_path};

#[derive(Serialize)]
pub(super) struct StartedRecord<'a> {
    pub(super) version: u16,
    pub(super) sequence: u64,
    pub(super) event: &'static str,
    pub(super) command: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) invocation_id: Option<&'a str>,
}

#[derive(Serialize)]
pub(super) struct ResultRecord {
    pub(super) version: u16,
    pub(super) sequence: u64,
    pub(super) event: &'static str,
    pub(super) command: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) invocation_id: Option<String>,
    pub(super) status: ResultStatus,
    pub(super) exit_code: u8,
    pub(super) data: CommandData,
}

#[derive(Serialize)]
pub(super) struct ErrorRecord {
    pub(super) version: u16,
    pub(super) sequence: u64,
    pub(super) event: &'static str,
    pub(super) command: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) invocation_id: Option<String>,
    pub(super) status: ErrorStatus,
    pub(super) exit_code: u8,
    pub(super) error: StructuredError,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ErrorStatus {
    Failure,
}

#[derive(Serialize)]
#[serde(untagged)]
pub(super) enum CommandData {
    Success(Box<SuccessData>),
    ContentDiagnostics(ContentDiagnosticData),
}

/// Successful command payloads. Each output phase has its own variant, so a
/// successful compile always has an artifact and extract cannot expose both
/// an artifact and catalog entries.
#[derive(Serialize)]
#[serde(untagged)]
pub(super) enum SuccessData {
    Validate {
        diagnostics: Vec<DiagnosticRecord>,
    },
    Compile {
        diagnostics: Vec<DiagnosticRecord>,
        artifact: ArtifactMetadata,
    },
    ExtractArtifact {
        diagnostics: Vec<DiagnosticRecord>,
        artifact: ArtifactMetadata,
    },
    ExtractEntries {
        diagnostics: Vec<DiagnosticRecord>,
        entries: Vec<CatalogEntry>,
    },
    Runtime {
        trace: TraceDocument,
    },
}

/// Content failures are still successful protocol operations: diagnostics are
/// returned in `command.result` with exit code 1 instead of `command.error`.
#[derive(Serialize)]
#[serde(untagged)]
pub(super) enum ContentDiagnosticData {
    Validate { diagnostics: Vec<DiagnosticRecord> },
    Compile { diagnostics: Vec<DiagnosticRecord> },
    Extract { diagnostics: Vec<DiagnosticRecord> },
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ResultStatus {
    Success,
    ContentDiagnostics,
}

impl ResultStatus {
    pub(super) const fn exit_code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::ContentDiagnostics => 1,
        }
    }
}

pub(super) enum StructuredOutcome {
    Success(Box<SuccessData>),
    ContentDiagnostics(ContentDiagnosticData),
}

impl StructuredOutcome {
    pub(super) fn success(data: SuccessData) -> Self {
        Self::Success(Box::new(data))
    }

    pub(super) fn content_diagnostics(data: ContentDiagnosticData) -> Self {
        Self::ContentDiagnostics(data)
    }

    pub(super) const fn exit_code(&self) -> u8 {
        match self {
            Self::Success(_) => 0,
            Self::ContentDiagnostics(_) => 1,
        }
    }

    pub(super) fn into_parts(self) -> (ResultStatus, CommandData) {
        match self {
            Self::Success(data) => (ResultStatus::Success, CommandData::Success(data)),
            Self::ContentDiagnostics(data) => (
                ResultStatus::ContentDiagnostics,
                CommandData::ContentDiagnostics(data),
            ),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct ArtifactMetadata {
    pub(crate) path: MachinePathProjection,
    pub(crate) size_bytes: u64,
}

pub(crate) fn artifact_metadata(path: &Path) -> Result<ArtifactMetadata, CliError> {
    let size_bytes = fs::metadata(path)
        .map_err(|source| CliError::AssetMetadata {
            path: path.to_owned(),
            source,
        })?
        .len();
    Ok(ArtifactMetadata {
        path: machine_path(path),
        size_bytes,
    })
}

#[derive(Serialize)]
pub(super) struct CatalogEntry {
    pub(super) context: String,
    pub(super) source_text: String,
    pub(super) plural_source_text: Option<String>,
    pub(super) comments: Vec<String>,
    pub(super) reference: Option<CatalogReference>,
}

impl From<&PotEntry> for CatalogEntry {
    fn from(entry: &PotEntry) -> Self {
        Self {
            context: entry.context.clone(),
            source_text: entry.source_text.clone(),
            plural_source_text: entry.plural_source_text.clone(),
            comments: entry.comments.clone(),
            reference: entry.reference.as_ref().map(CatalogReference::from),
        }
    }
}

#[derive(Serialize)]
pub(super) struct CatalogReference {
    pub(super) file: String,
    pub(super) line: u32,
    pub(super) column: u32,
}

impl From<&recite_compiler::PotReference> for CatalogReference {
    fn from(reference: &recite_compiler::PotReference) -> Self {
        Self {
            file: reference.file.clone(),
            line: reference.line,
            column: reference.column,
        }
    }
}

pub(crate) fn diagnostic_records(
    diagnostics: &[Diagnostic],
) -> Result<Vec<DiagnosticRecord>, CliError> {
    diagnostics
        .iter()
        .map(|diagnostic| {
            diagnostic
                .record()
                .map_err(|source| CliError::DiagnosticRendering {
                    source: source.to_string(),
                })
        })
        .collect()
}
