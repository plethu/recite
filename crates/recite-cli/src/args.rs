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
    Validate(InputArgs),
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
    Explain(ExplainArgs),
    Watch(WatchArgs),
    Run(RuntimeArgs),
    Trace(TraceArgs),
    Play(PlayArgs),
}

#[derive(Debug, Args)]
pub(crate) struct InputArgs {
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
pub(crate) struct WatchArgs {
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
    #[arg(required = true)]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct ExtractArgs {
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,
    #[arg(long)]
    pub(crate) schema: Option<PathBuf>,
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
