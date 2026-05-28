use std::path::PathBuf;

use clap::Parser;
use recite_fixturegen::{
    FixtureConfigSet, GenerateMode, SummarySet, write_profile_summary, write_project,
    write_summaries,
};

#[derive(Debug, Parser)]
#[command(name = "recite-fixturegen", version)]
struct Args {
    #[arg(long, default_value = "fixtures/synthetic/profiles.toml")]
    profiles: PathBuf,
    #[arg(long)]
    profile: Option<String>,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    summaries: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = GenerateMode::Project)]
    mode: GenerateMode,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let profiles = FixtureConfigSet::load_path(&args.profiles)?;

    match (args.profile.as_deref(), args.mode) {
        (Some(profile), GenerateMode::Project) => {
            let output = args.output.ok_or("--output is required for project mode")?;
            let config = profiles.profile(profile)?;
            let summary = write_project(config, &output)?;
            if let Some(summary_path) = args.summaries {
                write_profile_summary(&summary, &summary_path)?;
            }
        }
        (Some(profile), GenerateMode::Summary) => {
            let output = args
                .summaries
                .ok_or("--summaries is required for summary mode")?;
            let config = profiles.profile(profile)?;
            let summary = SummarySet::generate_one(config)?;
            write_profile_summary(&summary, &output)?;
        }
        (None, GenerateMode::Summary) => {
            let output = args
                .summaries
                .ok_or("--summaries is required for summary mode")?;
            let summaries = SummarySet::generate_all(&profiles)?;
            write_summaries(&summaries, &output)?;
        }
        (None, GenerateMode::Project) => {
            return Err("--profile is required for project mode".into());
        }
    }

    Ok(())
}
