use std::ffi::OsString;
use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;

mod args;
mod commands;
mod diagnostics;
mod error;
mod fs;
mod runtime_fixture;

use args::Cli;
use error::CliError;

const SUCCESS: ExitCode = ExitCode::SUCCESS;

pub fn run(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    let cli = Cli::parse_from(args);
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();

    match commands::run_command(cli.command, &mut stdout, &mut stderr) {
        Ok(()) => SUCCESS,
        Err(CliError::Diagnostics) => ExitCode::from(1),
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            ExitCode::from(1)
        }
    }
}
