//! Gungraun owns measurement and budgets; this tool rejects incomplete comparisons.
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use recite_perf::{Scope, check_summaries};

mod prepare;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Check {
        #[arg(value_enum)]
        scope: SelectedScope,
        summaries: PathBuf,
    },
    /// Add the current private harness to an archived control workspace.
    Prepare {
        candidate: PathBuf,
        control: PathBuf,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum SelectedScope {
    Pr,
    Full,
    Writer,
    BoundText,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    match args.command {
        Command::Prepare { candidate, control } => prepare::workspace(&candidate, &control)?,
        Command::Check { scope, summaries } => {
            let scope = match scope {
                SelectedScope::Pr => Scope::Pr,
                SelectedScope::Full => Scope::Full,
                SelectedScope::Writer => Scope::Writer,
                SelectedScope::BoundText => Scope::BoundText,
            };
            check_summaries(BufReader::new(File::open(summaries)?), scope)?;
            println!("Every required workload has paired instruction and heap evidence.");
        }
    }
    Ok(())
}
