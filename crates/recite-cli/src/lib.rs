use std::ffi::OsString;
use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;

mod args;
mod commands;
mod diagnostics;
mod error;
mod fs;
mod i18n;
mod play;
mod runtime_fixture;
mod runtime_format;
mod tui;

use args::Cli;
use error::CliError;
use i18n::{Messages, UiLocale};

const SUCCESS: ExitCode = ExitCode::SUCCESS;

pub fn run(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    let cli = Cli::parse_from(args);
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();

    match commands::run_command(cli.command, &mut stdout, &mut stderr) {
        Ok(()) => SUCCESS,
        Err(CliError::Diagnostics) => ExitCode::from(1),
        Err(error) => {
            let message = Messages::load(&UiLocale::default())
                .map(|messages| error.to_user_message(&messages))
                .unwrap_or_else(|_| error.to_string());
            let _ = writeln!(stderr, "error: {message}");
            ExitCode::from(1)
        }
    }
}
