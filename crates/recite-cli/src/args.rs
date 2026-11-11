use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use recite_core::DiagnosticCategory;

#[derive(Debug, Parser)]
#[command(name = "recite", version)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    Validate(ValidateArgs),
    Compile(CompileArgs),
    Extract(ExtractArgs),
    /// Dialogue PO catalogues and their review state.
    #[command(subcommand)]
    Catalog(CatalogCommand),
    /// Stable line and choice IDs.
    #[command(subcommand)]
    Id(IdCommand),
    /// Schema manifests and their producers.
    #[command(subcommand)]
    Schema(SchemaCommand),
    /// Project manifests and compiled-asset freshness.
    #[command(subcommand)]
    Project(ProjectCommand),
    Explain(ExplainArgs),
    Watch(WatchArgs),
    Run(RuntimeArgs),
    Trace(TraceArgs),
    Play(PlayArgs),
    Bench(BenchArgs),
    /// Inspect a bounded migration; write only with --output-dir.
    Import(crate::migration::ImportArgs),
}

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
    #[arg(long, value_enum, default_value_t = CatalogFormat::Text)]
    pub(crate) format: CatalogFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub(crate) enum CatalogFormat {
    Text,
    Json,
}

#[derive(Debug, Subcommand)]
pub(crate) enum IdCommand {
    Check(CheckIdsArgs),
    Assign(InputArgs),
    Rename(RenameIdArgs),
    Reassign(ReassignIdArgs),
}

#[derive(Debug, Subcommand)]
pub(crate) enum SchemaCommand {
    Inspect(InspectSchemaArgs),
    Export(ExportSchemaArgs),
    #[command(name = "check-fresh")]
    CheckFresh(ProducerFreshnessArgs),
}

#[derive(Debug, Subcommand)]
pub(crate) enum ProjectCommand {
    Validate(ProjectRootArgs),
    #[command(name = "check-fresh")]
    CheckFresh(ProjectRootArgs),
}

#[derive(Debug, Args)]
pub(crate) struct InputArgs {
    #[arg(required = true)]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct CheckIdsArgs {
    #[command(flatten)]
    pub(crate) inputs: InputArgs,
    /// Git revision whose stable IDs must still exist, such as the last release tag.
    #[arg(long, value_name = "REF")]
    pub(crate) since: Option<String>,
    /// Stable ID intentionally removed since `--since`; repeat for each ID.
    #[arg(long = "allow-removed", value_name = "ID", requires = "since")]
    pub(crate) allow_removed: Vec<String>,
}

/// The project an ID edit reads: explicit sources, or the nearest manifest's.
#[derive(Debug, Args)]
pub(crate) struct IdProjectArgs {
    /// `.recite` files or directories forming the whole project.
    #[arg(value_name = "PATHS")]
    pub(crate) paths: Vec<PathBuf>,
    /// Dialogue PO catalogue to update besides `[discovery].catalogs`.
    #[arg(long = "catalog", value_name = "PATH")]
    pub(crate) catalogs: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct RenameIdArgs {
    /// Stable ID to change, or `PATH:LINE` of its header.
    #[arg(value_name = "TARGET")]
    pub(crate) target: String,
    /// New stable ID.
    #[arg(value_name = "NEW_ID")]
    pub(crate) new_id: String,
    #[command(flatten)]
    pub(crate) project: IdProjectArgs,
}

#[derive(Debug, Args)]
pub(crate) struct ReassignIdArgs {
    /// Stable ID to change, or `PATH:LINE` of its header.
    #[arg(value_name = "TARGET")]
    pub(crate) target: String,
    #[command(flatten)]
    pub(crate) project: IdProjectArgs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub(crate) enum OutputFormat {
    Human,
    Structured,
}

/// Diagnostic categories `validate --only` can focus on.
#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub(crate) enum ValidateOnly {
    Markup,
    Metadata,
}

impl ValidateOnly {
    pub(crate) const fn category(self) -> DiagnosticCategory {
        match self {
            Self::Markup => DiagnosticCategory::Markup,
            Self::Metadata => DiagnosticCategory::Metadata,
        }
    }
}

#[derive(Debug, Args)]
pub(crate) struct ValidateArgs {
    /// Schema manifest whose declarations, metadata and markup policy are checked.
    #[arg(long, required_if_eq("only", "metadata"))]
    pub(crate) schema: Option<PathBuf>,
    /// Report only this diagnostic category; syntax errors are always reported.
    #[arg(long, value_enum, value_name = "CATEGORY")]
    pub(crate) only: Option<ValidateOnly>,
    /// Select human-readable or version-1 newline-delimited structured output.
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub(crate) output_format: OutputFormat,
    /// Caller-owned identifier copied into each structured protocol record.
    #[arg(long)]
    pub(crate) invocation_id: Option<String>,
    #[arg(required = true)]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct ProjectRootArgs {
    pub(crate) project_root: PathBuf,
}

#[derive(Debug, Args)]
pub(crate) struct ProducerFreshnessArgs {
    /// Previously exported manifest containing expected producer fingerprints.
    #[arg(long)]
    pub(crate) expected: PathBuf,
    /// Current producer export to compare with the expected manifest.
    #[arg(long)]
    pub(crate) actual: PathBuf,
}

#[derive(Debug, Args)]
pub(crate) struct InspectSchemaArgs {
    /// Standalone Recite TOML or generated schema manifest JSON.
    pub(crate) schema: PathBuf,
}

#[derive(Debug, Args)]
pub(crate) struct ExportSchemaArgs {
    /// Standalone Recite TOML or generated schema manifest JSON.
    #[arg(long)]
    pub(crate) schema: PathBuf,
    /// Canonical generated schema manifest JSON destination.
    #[arg(long)]
    pub(crate) output: PathBuf,
    /// Engine producer kind that owns the generated manifest.
    #[arg(long, requires = "producer_id")]
    pub(crate) producer_kind: Option<String>,
    /// Stable engine registration or resource ID, independent of file path.
    #[arg(long, requires = "producer_kind")]
    pub(crate) producer_id: Option<String>,
    /// Select human-readable or version-1 newline-delimited structured output.
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub(crate) output_format: OutputFormat,
    /// Caller-owned identifier copied into each structured protocol record.
    #[arg(long)]
    pub(crate) invocation_id: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct WatchArgs {
    /// Select human-readable or version-1 newline-delimited structured output.
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub(crate) output_format: OutputFormat,
    /// Caller-owned identifier copied into each structured protocol record.
    #[arg(long)]
    pub(crate) invocation_id: Option<String>,
    pub(crate) project_root: PathBuf,
}

#[derive(Debug, Args)]
pub(crate) struct ExplainArgs {
    pub(crate) code: String,
}

#[derive(Debug, Args)]
pub(crate) struct CompileArgs {
    #[arg(short, long)]
    pub(crate) output: PathBuf,
    #[arg(long)]
    pub(crate) schema: Option<PathBuf>,
    /// Select human-readable or version-1 newline-delimited structured output.
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub(crate) output_format: OutputFormat,
    /// Caller-owned identifier copied into each structured protocol record.
    #[arg(long)]
    pub(crate) invocation_id: Option<String>,
    #[arg(required = true)]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct ExtractArgs {
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,
    #[arg(long)]
    pub(crate) schema: Option<PathBuf>,
    /// Select human-readable or version-1 newline-delimited structured output.
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub(crate) output_format: OutputFormat,
    /// Caller-owned identifier copied into each structured protocol record.
    #[arg(long)]
    pub(crate) invocation_id: Option<String>,
    #[arg(required = true)]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct RuntimeArgs {
    pub(crate) asset: PathBuf,
    #[arg(long)]
    pub(crate) block: String,
    #[arg(long)]
    pub(crate) fixture: PathBuf,
    /// Select human-readable or version-1 newline-delimited structured output.
    #[arg(long, value_enum, default_value_t = OutputFormat::Human)]
    pub(crate) output_format: OutputFormat,
    /// Caller-owned identifier copied into each structured protocol record.
    #[arg(long)]
    pub(crate) invocation_id: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct TraceArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    #[arg(long)]
    pub(crate) metrics: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub(crate) enum PlayUi {
    Auto,
    Tui,
    Plain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub(crate) enum PlayKeymap {
    Standard,
    Vim,
}

#[derive(Debug, Args)]
pub(crate) struct PlayArgs {
    pub(crate) asset: PathBuf,
    #[arg(long)]
    pub(crate) block: String,
    #[arg(long, value_enum, default_value_t = PlayUi::Auto)]
    pub(crate) ui: PlayUi,
    #[arg(long, value_enum)]
    pub(crate) keymap: Option<PlayKeymap>,
    #[arg(long)]
    pub(crate) dialogue_locale: Option<String>,
    #[arg(long)]
    pub(crate) dialogue_catalog: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub(crate) enum BenchFormat {
    Json,
    Markdown,
}

#[derive(Debug, Args)]
pub(crate) struct BenchArgs {
    pub(crate) target: String,
    #[arg(long)]
    pub(crate) scale: Vec<String>,
    #[arg(long)]
    pub(crate) group: Vec<String>,
    #[arg(long, value_enum, default_value_t = BenchFormat::Markdown)]
    pub(crate) format: BenchFormat,
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    #[arg(long)]
    pub(crate) baseline: Option<PathBuf>,
    #[arg(long, default_value_t = 3)]
    pub(crate) samples: usize,
}
