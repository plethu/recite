//! Arguments for `catalog` and `recording`: dialogue catalogues and recording handoffs.

use std::path::PathBuf;

use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub(crate) enum CatalogCommand {
    Status(CatalogStatusArgs),
    Refresh(CatalogProjectArgs),
}

/// The project whose sources and catalogues are compared.
#[derive(Debug, Args)]
pub(crate) struct CatalogProjectArgs {
    /// Project root containing recite.project.toml.
    #[arg(value_name = "PROJECT_ROOT", default_value = ".")]
    pub(crate) project_root: PathBuf,
    /// Dialogue PO catalogue to include besides `[discovery].catalogs`.
    #[arg(long = "catalog", value_name = "PATH")]
    pub(crate) catalogs: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct CatalogStatusArgs {
    #[command(flatten)]
    pub(crate) project: CatalogProjectArgs,
    /// Fail when an entry needs attention or a catalogue needs refreshing.
    #[arg(long)]
    pub(crate) check: bool,
    #[arg(long, value_enum, default_value_t = ReportFormat::Text)]
    pub(crate) format: ReportFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub(crate) enum ReportFormat {
    Text,
    Json,
}

#[derive(Debug, Subcommand)]
pub(crate) enum RecordingCommand {
    Status(RecordingStatusArgs),
    Export(RecordingExportArgs),
    Import(RecordingImportArgs),
}

/// The recording language: the source, or one dialogue locale with a catalogue.
#[derive(Debug, Args)]
pub(crate) struct RecordingLanguageArgs {
    /// Dialogue locale to record; omit it for the source language.
    #[arg(long)]
    pub(crate) locale: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct RecordingStatusArgs {
    /// Project root containing recite.project.toml.
    #[arg(value_name = "PROJECT_ROOT", default_value = ".")]
    pub(crate) project_root: PathBuf,
    #[command(flatten)]
    pub(crate) language: RecordingLanguageArgs,
    /// Fail when any item needs recording attention.
    #[arg(long)]
    pub(crate) check: bool,
    #[arg(long, value_enum, default_value_t = ReportFormat::Text)]
    pub(crate) format: ReportFormat,
}

#[derive(Debug, Args)]
pub(crate) struct RecordingExportArgs {
    /// Project root containing recite.project.toml.
    #[arg(value_name = "PROJECT_ROOT", default_value = ".")]
    pub(crate) project_root: PathBuf,
    #[command(flatten)]
    pub(crate) language: RecordingLanguageArgs,
    /// Export only items in this block. Repeatable.
    #[arg(long = "block", value_name = "BLOCK")]
    pub(crate) blocks: Vec<String>,
    /// Export items in this state; defaults to to-record and stale. Repeatable.
    #[arg(long = "state", value_enum, value_name = "STATE")]
    pub(crate) states: Vec<ExportState>,
    /// Name recorded with each returned take.
    #[arg(long, value_name = "NAME", default_value = "")]
    pub(crate) handoff: String,
    #[arg(long, value_enum, default_value_t = HandoffFormat::Csv)]
    pub(crate) format: HandoffFormat,
    /// Write the handoff here; without it nothing is written.
    #[arg(long, value_name = "PATH")]
    pub(crate) output: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct RecordingImportArgs {
    /// Returned handoff file.
    #[arg(value_name = "FILE")]
    pub(crate) file: PathBuf,
    /// Project root containing recite.project.toml.
    #[arg(value_name = "PROJECT_ROOT", default_value = ".")]
    pub(crate) project_root: PathBuf,
    #[command(flatten)]
    pub(crate) language: RecordingLanguageArgs,
    /// Handoff format; defaults to JSON for `.json` files and CSV otherwise.
    #[arg(long, value_enum)]
    pub(crate) format: Option<HandoffFormat>,
    /// Update the recording sidecar; without it the import is only validated.
    #[arg(long)]
    pub(crate) apply: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub(crate) enum HandoffFormat {
    Csv,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub(crate) enum ExportState {
    ToRecord,
    Stale,
    Recorded,
    Approved,
}
