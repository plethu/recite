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

use args::{Cli, Command};
use error::CliError;
use i18n::{Messages, UiLocale};
use tui::TuiSettings;

const SUCCESS: ExitCode = ExitCode::SUCCESS;

pub fn run(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    let cli = Cli::parse_from(args);
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    let error_messages = error_messages_for_command(&cli.command);

    match commands::run_command(cli.command, &mut stdout, &mut stderr) {
        Ok(()) => SUCCESS,
        Err(CliError::Diagnostics) => ExitCode::from(1),
        Err(error) => {
            let message = error.to_user_message(&error_messages);
            let _ = writeln!(stderr, "error: {message}");
            ExitCode::from(1)
        }
    }
}

fn error_messages_for_command(command: &Command) -> Messages {
    match command {
        Command::Play(args) => TuiSettings::load(args.keymap)
            .and_then(|settings| Messages::load(&settings.locale))
            .unwrap_or_else(|_| default_messages()),
        _ => default_messages(),
    }
}

fn default_messages() -> Messages {
    Messages::load(&UiLocale::default()).expect("embedded default UI catalog must load")
}
