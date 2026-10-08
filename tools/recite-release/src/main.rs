mod inventory;
mod project;
mod receipt;
mod version;

use clap::{Parser, Subcommand};
use std::{error::Error, path::PathBuf, process::ExitCode};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Parser)]
#[command(about = "Validate Recite release identity and candidate artifacts")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Version {
        version: String,
    },
    Candidate {
        version: String,
        commit: String,
    },
    Tag {
        version: String,
        commit: String,
    },
    Seal {
        version: String,
        commit: String,
        plan: PathBuf,
        artifacts: PathBuf,
        output: PathBuf,
    },
    Verify {
        version: String,
        commit: String,
        receipt: PathBuf,
        artifacts: PathBuf,
    },
    Assets {
        version: String,
        commit: String,
        receipt: PathBuf,
        artifacts: PathBuf,
        output: PathBuf,
    },
    Provenance {
        receipt: PathBuf,
        run: String,
        workflow_commit: String,
    },
}

fn run(command: Command) -> Result<()> {
    match command {
        Command::Version { version } => {
            let parsed = version::parse(&version)?;
            println!(
                "{}",
                serde_json::json!({"version": parsed.to_string(), "prerelease": !parsed.pre.is_empty()})
            );
        }
        Command::Candidate { version, commit } => project::candidate(&version, &commit)?,
        Command::Tag { version, commit } => project::tag(&version, &commit)?,
        Command::Seal {
            version,
            commit,
            plan,
            artifacts,
            output,
        } => {
            project::candidate(&version, &commit)?;
            receipt::seal(&version, &commit, &plan, &artifacts, &output)?;
        }
        Command::Verify {
            version,
            commit,
            receipt,
            artifacts,
        } => {
            receipt::verify(&version, &commit, &receipt, &artifacts)?;
        }
        Command::Assets {
            version,
            commit,
            receipt,
            artifacts,
            output,
        } => {
            receipt::assets(&version, &commit, &receipt, &artifacts, &output)?;
        }
        Command::Provenance {
            receipt,
            run,
            workflow_commit,
        } => {
            receipt::provenance(&receipt, &run, &workflow_commit)?;
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
