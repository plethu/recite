use std::ffi::OsString;

use clap::{Arg, ArgAction, CommandFactory, Error, FromArgMatches};

use crate::args::Cli;
use crate::i18n::{Messages, MsgId, UiLocale};
use crate::tui::TuiSettings;

pub(crate) fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Cli, Error> {
    let args = args.into_iter().collect::<Vec<_>>();
    let messages = if requests_help(&args) {
        help_messages()
    } else {
        default_messages()
    };
    let mut command = Cli::command();
    localise_command(&mut command, &messages);
    let matches = command.try_get_matches_from_mut(args)?;
    Cli::from_arg_matches(&matches).map_err(|error| error.format(&mut command))
}

// Invariant: the embedded default UI catalog is bundled with the CLI binary.
#[allow(clippy::expect_used)]
fn help_messages() -> Messages {
    Messages::load(&TuiSettings::help_locale())
        .or_else(|_| Messages::load(&UiLocale::default()))
        .expect("embedded default UI catalog must load")
}

fn localise_command(command: &mut clap::Command, messages: &Messages) {
    *command = std::mem::take(command)
        .about(messages.text(MsgId::CliHelpAbout))
        .help_template(help_template(messages))
        .subcommand_help_heading(messages.text(MsgId::CliHelpCommandsHeading))
        .next_help_heading(messages.text(MsgId::CliHelpOptionsHeading))
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(help_arg(messages))
        .arg(version_arg(messages));

    for subcommand in command.get_subcommands_mut() {
        localise_subcommand(subcommand, messages);
    }
}

fn localise_subcommand(command: &mut clap::Command, messages: &Messages) {
    *command = std::mem::take(command)
        .help_template(help_template(messages))
        .next_help_heading(messages.text(MsgId::CliHelpOptionsHeading))
        .disable_help_flag(true)
        .arg(help_arg(messages));

    match command.get_name() {
        "validate" => {
            set_about(command, messages.text(MsgId::CliHelpCommandValidate));
            localise_paths(command, messages);
        }
        "compile" => {
            set_about(command, messages.text(MsgId::CliHelpCommandCompile));
            set_arg_help(
                command,
                "output",
                messages.text(MsgId::CliHelpArgOutputCompile),
                messages,
            );
            localise_schema(command, messages);
            localise_paths(command, messages);
        }
        "extract" => {
            set_about(command, messages.text(MsgId::CliHelpCommandExtract));
            set_arg_help(
                command,
                "output",
                messages.text(MsgId::CliHelpArgOutputExtract),
                messages,
            );
            localise_schema(command, messages);
            localise_paths(command, messages);
        }
        "check-ids" => {
            set_about(command, messages.text(MsgId::CliHelpCommandCheckIds));
            localise_paths(command, messages);
        }
        "check-markup" => {
            set_about(command, messages.text(MsgId::CliHelpCommandCheckMarkup));
            localise_schema(command, messages);
            localise_paths(command, messages);
        }
        "check-metadata" => {
            set_about(command, messages.text(MsgId::CliHelpCommandCheckMetadata));
            localise_schema(command, messages);
            localise_paths(command, messages);
        }
        "validate-project" => {
            set_about(command, messages.text(MsgId::CliHelpCommandValidateProject));
            set_arg_help(
                command,
                "project_root",
                messages.text(MsgId::CliHelpArgProjectRoot),
                messages,
            );
        }
        "check-fresh" => {
            set_about(command, messages.text(MsgId::CliHelpCommandCheckFresh));
            set_arg_help(
                command,
                "project_root",
                messages.text(MsgId::CliHelpArgProjectRoot),
                messages,
            );
        }
        "explain" => {
            set_about(command, messages.text(MsgId::CliHelpCommandExplain));
            set_arg_help(
                command,
                "code",
                messages.text(MsgId::CliHelpArgDiagnosticCode),
                messages,
            );
        }
        "watch" => {
            set_about(command, messages.text(MsgId::CliHelpCommandWatch));
            set_arg_help(
                command,
                "project_root",
                messages.text(MsgId::CliHelpArgProjectRoot),
                messages,
            );
        }
        "run" => {
            set_about(command, messages.text(MsgId::CliHelpCommandRun));
            localise_runtime_args(command, messages, MsgId::CliHelpArgAssetRun);
        }
        "trace" => {
            set_about(command, messages.text(MsgId::CliHelpCommandTrace));
            localise_runtime_args(command, messages, MsgId::CliHelpArgAssetRun);
        }
        "play" => {
            set_about(command, messages.text(MsgId::CliHelpCommandPlay));
            set_arg_help(
                command,
                "asset",
                messages.text(MsgId::CliHelpArgAssetPlay),
                messages,
            );
            set_arg_help(
                command,
                "block",
                messages.text(MsgId::CliHelpArgBlock),
                messages,
            );
            set_arg_help(command, "ui", messages.text(MsgId::CliHelpArgUi), messages);
            set_arg_help(
                command,
                "keymap",
                messages.text(MsgId::CliHelpArgKeymap),
                messages,
            );
            set_arg_help(
                command,
                "dialogue_locale",
                messages.text(MsgId::CliHelpArgDialogueLocale),
                messages,
            );
            set_arg_help(
                command,
                "dialogue_catalog",
                messages.text(MsgId::CliHelpArgDialogueCatalog),
                messages,
            );
        }
        "bench" => {
            set_about(command, messages.text(MsgId::CliHelpCommandBench));
        }
        _ => {}
    }
}

fn localise_paths(command: &mut clap::Command, messages: &Messages) {
    set_arg_help(
        command,
        "paths",
        messages.text(MsgId::CliHelpArgPaths),
        messages,
    );
}

fn localise_schema(command: &mut clap::Command, messages: &Messages) {
    set_arg_help(
        command,
        "schema",
        messages.text(MsgId::CliHelpArgSchema),
        messages,
    );
}

fn localise_runtime_args(command: &mut clap::Command, messages: &Messages, asset: MsgId) {
    set_arg_help(command, "asset", messages.text(asset), messages);
    set_arg_help(
        command,
        "block",
        messages.text(MsgId::CliHelpArgBlock),
        messages,
    );
    set_arg_help(
        command,
        "fixture",
        messages.text(MsgId::CliHelpArgFixture),
        messages,
    );
}

fn set_arg_help(command: &mut clap::Command, id: &'static str, help: String, messages: &Messages) {
    let heading = if id == "paths" || id == "project_root" || id == "asset" || id == "code" {
        messages.text(MsgId::CliHelpArgumentsHeading)
    } else {
        messages.text(MsgId::CliHelpOptionsHeading)
    };
    *command = std::mem::take(command).mut_arg(id, |arg| arg.help(help).help_heading(heading));
}

fn set_about(command: &mut clap::Command, about: String) {
    *command = std::mem::take(command).about(about);
}

fn help_template(messages: &Messages) -> String {
    format!(
        "{{about}}\n\n{} {{usage}}\n\n{{all-args}}{{after-help}}",
        messages.text(MsgId::CliHelpUsageHeading)
    )
}

fn help_arg(messages: &Messages) -> Arg {
    Arg::new("help")
        .short('h')
        .long("help")
        .action(ArgAction::Help)
        .help(messages.text(MsgId::CliHelpArgHelp))
        .help_heading(messages.text(MsgId::CliHelpOptionsHeading))
}

fn version_arg(messages: &Messages) -> Arg {
    Arg::new("version")
        .short('V')
        .long("version")
        .action(ArgAction::Version)
        .help(messages.text(MsgId::CliHelpArgVersion))
        .help_heading(messages.text(MsgId::CliHelpOptionsHeading))
}

// Invariant: the embedded default UI catalog is bundled with the CLI binary.
#[allow(clippy::expect_used)]
fn default_messages() -> Messages {
    Messages::load(&UiLocale::default()).expect("embedded default UI catalog must load")
}

fn requests_help(args: &[OsString]) -> bool {
    for (index, arg) in args.iter().skip(1).enumerate() {
        match arg.to_str() {
            Some("help") if index == 0 => return true,
            Some("-h" | "--help") => return true,
            Some("--") => return false,
            _ => {}
        }
    }
    false
}
