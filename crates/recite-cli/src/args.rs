use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "recite",
    version,
    about = "Recite dialogue compiler and validation CLI."
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Validate dialogue source without writing compiled output.
    Validate(InputArgs),
    /// Compile dialogue source to a MessagePack .recitec asset.
    Compile(CompileArgs),
    /// Extract gettext POT entries.
    Extract(ExtractArgs),
    /// Report stable line and choice ID diagnostics.
    #[command(name = "check-ids")]
    CheckIds(InputArgs),
    /// Validate inline markup, optionally against a schema manifest.
    #[command(name = "check-markup")]
    CheckMarkup(SchemaInputArgs),
    /// Validate metadata against a schema manifest.
    #[command(name = "check-metadata")]
    CheckMetadata(RequiredSchemaInputArgs),
    /// Validate recite.project.toml and referenced compiled assets.
    #[command(name = "validate-project")]
    ValidateProject(ProjectRootArgs),
    /// Check whether project compiled assets are fresh.
    #[command(name = "check-fresh")]
    CheckFresh(ProjectRootArgs),
    /// Run a compiled asset headlessly with fixture data.
    Run(RuntimeArgs),
    /// Emit deterministic JSON for a headless fixture run.
    Trace(RuntimeArgs),
    /// Play a compiled asset interactively.
    Play(PlayArgs),
}

#[derive(Debug, Args)]
pub(crate) struct InputArgs {
    /// One or more .recite files, or directories containing .recite files.
    #[arg(required = true)]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct SchemaInputArgs {
    /// Generated schema manifest JSON.
    #[arg(long)]
    pub(crate) schema: Option<PathBuf>,
    /// One or more .recite files, or directories containing .recite files.
    #[arg(required = true)]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct RequiredSchemaInputArgs {
    /// Generated schema manifest JSON.
    #[arg(long)]
    pub(crate) schema: PathBuf,
    /// One or more .recite files, or directories containing .recite files.
    #[arg(required = true)]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct ProjectRootArgs {
    /// Project root containing recite.project.toml.
    pub(crate) project_root: PathBuf,
}

#[derive(Debug, Args)]
pub(crate) struct CompileArgs {
    /// Write MessagePack compiled asset bytes to this path.
    #[arg(short, long)]
    pub(crate) output: PathBuf,
    /// Generated schema manifest JSON.
    #[arg(long)]
    pub(crate) schema: Option<PathBuf>,
    /// One or more .recite files, or directories containing .recite files.
    #[arg(required = true)]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct ExtractArgs {
    /// Write POT output to this path instead of stdout.
    #[arg(short, long)]
    pub(crate) output: Option<PathBuf>,
    /// Generated schema manifest JSON.
    #[arg(long)]
    pub(crate) schema: Option<PathBuf>,
    /// One or more .recite files, or directories containing .recite files.
    #[arg(required = true)]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
pub(crate) struct RuntimeArgs {
    /// MessagePack .recitec asset to run.
    pub(crate) asset: PathBuf,
    /// Block ID to start from.
    #[arg(long)]
    pub(crate) block: String,
    /// TOML fixture with conditions, choices, and effect options.
    #[arg(long)]
    pub(crate) fixture: PathBuf,
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
    /// MessagePack .recitec asset to play.
    pub(crate) asset: PathBuf,
    /// Block ID to start from.
    #[arg(long)]
    pub(crate) block: String,
    /// Interactive UI mode.
    #[arg(long, value_enum, default_value_t = PlayUi::Auto)]
    pub(crate) ui: PlayUi,
    /// TUI keymap. Overrides [ui].keymap in the user config file.
    #[arg(long, value_enum)]
    pub(crate) keymap: Option<PlayKeymap>,
    /// Dialogue content locale to preview through the runtime locale provider.
    #[arg(long)]
    pub(crate) dialogue_locale: Option<String>,
    /// Dialogue gettext catalog mapping in LOCALE=PATH form. Repeatable.
    #[arg(long)]
    pub(crate) dialogue_catalog: Vec<String>,
}
