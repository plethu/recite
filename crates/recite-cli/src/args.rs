use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

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
    #[command(name = "check-ids")]
    CheckIds(InputArgs),
    #[command(name = "check-markup")]
    CheckMarkup(SchemaInputArgs),
    #[command(name = "check-metadata")]
    CheckMetadata(RequiredSchemaInputArgs),
    #[command(name = "validate-project")]
    ValidateProject(ProjectRootArgs),
    #[command(name = "check-fresh")]
    CheckFresh(ProjectRootArgs),
    #[command(name = "check-schema-producer-freshness")]
    CheckSchemaProducerFreshness(ProducerFreshnessArgs),
    #[command(name = "inspect-schema")]
    InspectSchema(InspectSchemaArgs),
    Explain(ExplainArgs),
    Watch(WatchArgs),
    Run(RuntimeArgs),
    Trace(TraceArgs),
    Play(PlayArgs),
    Bench(BenchArgs),
}

#[derive(Debug, Args)]
pub(crate) struct InputArgs {
    #[arg(required = true)]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum)]
pub(crate) enum OutputFormat {
    Human,
    Structured,
}

#[derive(Debug, Args)]
pub(crate) struct ValidateArgs {
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
pub(crate) struct SchemaInputArgs {
    #[arg(long)]
    pub(crate) schema: Option<PathBuf>,
    #[arg(required = true)]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct RequiredSchemaInputArgs {
    #[arg(long)]
    pub(crate) schema: PathBuf,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StructuredInvocation {
    pub(crate) command: &'static str,
    pub(crate) invocation_id: Option<String>,
}

impl Command {
    pub(crate) fn structured_invocation(&self) -> Option<StructuredInvocation> {
        let (command, output_format, invocation_id) = match self {
            Self::Validate(args) => ("validate", args.output_format, &args.invocation_id),
            Self::Compile(args) => ("compile", args.output_format, &args.invocation_id),
            Self::Extract(args) => ("extract", args.output_format, &args.invocation_id),
            Self::Watch(args) => ("watch", args.output_format, &args.invocation_id),
            Self::Run(args) => ("run", args.output_format, &args.invocation_id),
            Self::Trace(args) => (
                "trace",
                args.runtime.output_format,
                &args.runtime.invocation_id,
            ),
            _ => return None,
        };

        (output_format == OutputFormat::Structured).then(|| StructuredInvocation {
            command,
            invocation_id: invocation_id.clone(),
        })
    }
}
