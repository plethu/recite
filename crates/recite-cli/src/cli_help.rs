mod subcommands;
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

// Clap owns the syntax-error renderer (including its English labels and
// punctuation); its public API lets us localise command metadata and help,
// but not replace those built-in error templates. The CLI deliberately keeps
// this narrow host-owned residual rather than treating `Error::to_string()`
// as a Recite UI resource.

fn help_messages() -> Messages {
    // Recover through the required embedded default catalog when needed.
    Messages::load(&TuiSettings::help_locale()).unwrap_or_else(|_| default_messages())
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
        subcommands::localise(subcommand, messages);
    }
    group_subcommands(command, messages);
}

/// Top-level commands grouped as on the documentation's CLI page. Clap prints one subcommand
/// heading, so the groups are rendered into the template and the flat list is hidden.
const COMMAND_GROUPS: [(MsgId, &[&str]); 6] = [
    (
        MsgId::CliHelpGroupAuthoring,
        &[
            "validate",
            "check-ids",
            "check-markup",
            "check-metadata",
            "import",
        ],
    ),
    (
        MsgId::CliHelpGroupBuilding,
        &["compile", "validate-project", "check-fresh", "watch"],
    ),
    (MsgId::CliHelpGroupLocalisation, &["extract"]),
    (
        MsgId::CliHelpGroupRunning,
        &["run", "trace", "play", "bench"],
    ),
    (
        MsgId::CliHelpGroupSchema,
        &[
            "inspect-schema",
            "export-schema",
            "check-schema-producer-freshness",
        ],
    ),
    (MsgId::CliHelpGroupHelp, &["explain", "help"]),
];

fn group_subcommands(command: &mut clap::Command, messages: &Messages) {
    let abouts: Vec<(String, String)> = command
        .get_subcommands()
        .map(|sub| {
            (
                sub.get_name().to_owned(),
                sub.get_about().map(ToString::to_string).unwrap_or_default(),
            )
        })
        .collect();
    let about = |name: &str| {
        abouts
            .iter()
            .find(|(candidate, _)| candidate == name)
            .map(|(_, about)| about.clone())
            .unwrap_or_else(|| messages.text(MsgId::CliHelpArgHelp))
    };
    let width = COMMAND_GROUPS
        .iter()
        .flat_map(|(_, names)| names.iter())
        .map(|name| name.len())
        .max()
        .unwrap_or(0);
    let mut groups = String::new();
    let columns = crossterm::terminal::size()
        .map(|(width, _)| usize::from(width))
        .unwrap_or(80);
    for (heading, names) in COMMAND_GROUPS {
        groups.push_str(&format!("{}:\n", messages.text(heading)));
        for name in names {
            groups.push_str(&grouped_command(name, &about(name), width, columns));
            groups.push('\n');
        }
        groups.push('\n');
    }
    let grouped: Vec<&str> = COMMAND_GROUPS
        .iter()
        .flat_map(|(_, names)| names.iter().copied())
        .collect();
    let mut hidden = std::mem::take(command);
    for (name, _) in &abouts {
        if grouped.contains(&name.as_str()) {
            hidden = hidden.mut_subcommand(name, |sub| sub.hide(true));
        }
    }
    // Hidden subcommands drop `<COMMAND>` from the generated usage, so it is restated.
    let usage = format!("{} <COMMAND>", hidden.get_name());
    *command = hidden.override_usage(usage).help_template(format!(
        "{{about}}\n\n{} {{usage}}\n\n{groups}{{all-args}}{{after-help}}",
        messages.text(MsgId::CliHelpUsageHeading)
    ));
}

fn grouped_command(name: &str, about: &str, name_width: usize, columns: usize) -> String {
    let (prefix, indent) = if columns < name_width + 28 {
        ("    ".to_owned(), "    ".to_owned())
    } else {
        (
            format!("  {name:<name_width$}  "),
            " ".repeat(name_width + 4),
        )
    };
    let description = textwrap::fill(
        about,
        textwrap::Options::new(columns.max(1))
            .initial_indent(&prefix)
            .subsequent_indent(&indent),
    );
    if columns < name_width + 28 {
        format!("  {name}\n{description}")
    } else {
        description
    }
}

/// Give every argument an explicit localised heading. An argument without one lands in clap's
/// built-in "Options" section, which printed a second, identically named heading.
fn assign_headings(command: &mut clap::Command, messages: &Messages) {
    let arguments = messages.text(MsgId::CliHelpArgumentsHeading);
    let options = messages.text(MsgId::CliHelpOptionsHeading);
    *command = std::mem::take(command).mut_args(|arg| {
        if arg.get_help_heading().is_some() {
            return arg;
        }
        let heading = if arg.is_positional() {
            arguments.clone()
        } else {
            options.clone()
        };
        arg.help_heading(heading)
    });
    // Clap orders custom headings by argument order, and `mut_arg` moves an argument to the end.
    // Touch every option once more so the Arguments section prints before Options.
    let option_ids: Vec<String> = command
        .get_arguments()
        .filter(|arg| !arg.is_positional())
        .map(|arg| arg.get_id().to_string())
        .collect();
    for id in option_ids {
        *command = std::mem::take(command).mut_arg(id, |arg| arg);
    }
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
#[allow(
    clippy::expect_used,
    reason = "the embedded default UI catalog is a required CLI resource"
)]
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

#[cfg(test)]
mod tests;
