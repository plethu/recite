//! Arguments for `recite save`.

use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::ReportFormat;

#[derive(Debug, Subcommand)]
pub(crate) enum SaveCommand {
    Impact(SaveImpactArgs),
    Inspect(SaveInspectArgs),
}

#[derive(Debug, Args)]
pub(crate) struct SaveImpactArgs {
    /// Patched compiled asset, or project root to build.
    #[arg(value_name = "CANDIDATE", default_value = ".")]
    pub(crate) candidate: PathBuf,
    /// Compiled asset players' saves were taken against.
    #[arg(long, value_name = "ASSET", required_unless_present = "since")]
    pub(crate) baseline: Option<PathBuf>,
    /// Git revision whose project build players' saves were taken against.
    #[arg(long, value_name = "REF", conflicts_with = "baseline")]
    pub(crate) since: Option<String>,
    /// Fail when a save point falls back outside an `--allow-fallback` block.
    #[arg(long)]
    pub(crate) check: bool,
    /// Block whose save points may fall back; repeat for each block.
    #[arg(long = "allow-fallback", value_name = "BLOCK", requires = "check")]
    pub(crate) allow_fallback: Vec<String>,
    #[arg(long, value_enum, default_value_t = ReportFormat::Text)]
    pub(crate) format: ReportFormat,
}

#[derive(Debug, Args)]
pub(crate) struct SaveInspectArgs {
    /// Saved session bytes, as the game stored them.
    #[arg(value_name = "SAVE")]
    pub(crate) save: PathBuf,
    /// Compiled asset to predict the restore outcome against.
    #[arg(long, value_name = "ASSET")]
    pub(crate) asset: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = ReportFormat::Text)]
    pub(crate) format: ReportFormat,
}
