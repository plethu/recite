//! Localised command and argument metadata; clap owns parsing.
use super::{assign_headings, help_arg, help_template};
use crate::i18n::{Messages, MsgId};

mod recording;

/// Localise `command` and, for noun groups, everything beneath it. Entries are keyed by the full
/// command path (`schema check-fresh`), so the same verb can exist under two nouns.
pub(super) fn localise(command: &mut clap::Command, messages: &Messages, parent: &str) {
    let path = if parent.is_empty() {
        command.get_name().to_owned()
    } else {
        format!("{parent} {}", command.get_name())
    };
    *command = std::mem::take(command)
        .help_template(help_template(messages))
        .next_help_heading(messages.text(MsgId::CliHelpOptionsHeading))
        .disable_help_flag(true)
        .arg(help_arg(messages));

    match path.as_str() {
        "import" => {
            set_about(command, messages.text(MsgId::CliHelpCommandImport));
            set_positional_arg_help(
                command,
                "input",
                messages.text(MsgId::CliHelpArgImportInput),
                messages,
            );
            localise_schema(command, messages);
            for (id, message) in [
                ("from", MsgId::CliHelpArgImportFrom),
                ("mapping", MsgId::CliHelpArgImportMapping),
                ("output_dir", MsgId::CliHelpArgImportOutputDir),
                ("accept_partial", MsgId::CliHelpArgImportAcceptPartial),
            ] {
                set_arg_help(command, id, messages.text(message), messages);
            }
        }
        "validate" => {
            set_about(command, messages.text(MsgId::CliHelpCommandValidate));
            localise_schema(command, messages);
            set_arg_help(
                command,
                "only",
                messages.text(MsgId::CliHelpArgOnly),
                messages,
            );
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
        "id check" => {
            set_about(command, messages.text(MsgId::CliHelpCommandIdCheck));
            localise_paths(command, messages);
            set_arg_help(
                command,
                "since",
                messages.text(MsgId::CliHelpArgSince),
                messages,
            );
            set_arg_help(
                command,
                "allow_removed",
                messages.text(MsgId::CliHelpArgAllowRemoved),
                messages,
            );
        }
        "id assign" => {
            set_about(command, messages.text(MsgId::CliHelpCommandIdAssign));
            localise_paths(command, messages);
        }
        "id rename" | "id reassign" => {
            let rename = path == "id rename";
            let about = if rename {
                MsgId::CliHelpCommandIdRename
            } else {
                MsgId::CliHelpCommandIdReassign
            };
            set_about(command, messages.text(about));
            let mut positionals = vec![("target", MsgId::CliHelpArgIdTarget)];
            if rename {
                positionals.push(("new_id", MsgId::CliHelpArgNewId));
            }
            positionals.push(("paths", MsgId::CliHelpArgIdPaths));
            // `mut_arg` moves an argument to the end; keep usage order explicit.
            for (order, (id, help)) in positionals.into_iter().enumerate() {
                set_positional_arg_help(command, id, messages.text(help), messages);
                *command = std::mem::take(command).mut_arg(id, |arg| arg.display_order(order));
            }
            set_arg_help(
                command,
                "catalogs",
                messages.text(MsgId::CliHelpArgCatalog),
                messages,
            );
        }
        "project validate" => {
            set_about(command, messages.text(MsgId::CliHelpCommandProjectValidate));
            set_arg_help(
                command,
                "project_root",
                messages.text(MsgId::CliHelpArgProjectRoot),
                messages,
            );
        }
        "project check-fresh" => {
            set_about(
                command,
                messages.text(MsgId::CliHelpCommandProjectCheckFresh),
            );
            set_arg_help(
                command,
                "project_root",
                messages.text(MsgId::CliHelpArgProjectRoot),
                messages,
            );
        }
        "schema inspect" => {
            set_about(command, messages.text(MsgId::CliHelpCommandSchemaInspect));
            set_positional_arg_help(
                command,
                "schema",
                messages.text(MsgId::CliHelpArgSchemaInspection),
                messages,
            );
        }
        "schema export" => {
            set_about(command, messages.text(MsgId::CliHelpCommandSchemaExport));
            set_arg_help(
                command,
                "schema",
                messages.text(MsgId::CliHelpArgSchemaInspection),
                messages,
            );
            set_arg_help(
                command,
                "output",
                messages.text(MsgId::CliHelpArgOutputSchema),
                messages,
            );
            set_arg_help(
                command,
                "producer_kind",
                messages.text(MsgId::CliHelpArgProducerKind),
                messages,
            );
            set_arg_help(
                command,
                "producer_id",
                messages.text(MsgId::CliHelpArgProducerId),
                messages,
            );
        }
        "schema check-fresh" => {
            set_about(
                command,
                messages.text(MsgId::CliHelpCommandSchemaCheckFresh),
            );
        }
        "catalog status" | "catalog refresh" => {
            let about = if path == "catalog status" {
                MsgId::CliHelpCommandCatalogStatus
            } else {
                MsgId::CliHelpCommandCatalogRefresh
            };
            set_about(command, messages.text(about));
            set_positional_arg_help(
                command,
                "project_root",
                messages.text(MsgId::CliHelpArgProjectRoot),
                messages,
            );
            set_arg_help(
                command,
                "catalogs",
                messages.text(MsgId::CliHelpArgCatalogInclude),
                messages,
            );
            if path == "catalog status" {
                set_arg_help(
                    command,
                    "check",
                    messages.text(MsgId::CliHelpArgCatalogCheck),
                    messages,
                );
                set_arg_help(
                    command,
                    "format",
                    messages.text(MsgId::CliHelpArgCatalogFormat),
                    messages,
                );
            }
        }
        "catalog" => localise_group(command, messages, MsgId::CliHelpCommandCatalog, &path),
        "recording" => localise_group(command, messages, MsgId::CliHelpCommandRecording, &path),
        "recording status" | "recording export" | "recording import" => {
            recording::localise(command, messages, &path);
        }
        "id" => localise_group(command, messages, MsgId::CliHelpCommandId, &path),
        "project" => localise_group(command, messages, MsgId::CliHelpCommandProject, &path),
        "schema" => localise_group(command, messages, MsgId::CliHelpCommandSchema, &path),
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
            set_arg_help(
                command,
                "scale",
                messages.text(MsgId::CliHelpArgBenchScale),
                messages,
            );
            set_arg_help(
                command,
                "group",
                messages.text(MsgId::CliHelpArgBenchGroup),
                messages,
            );
            set_arg_help(
                command,
                "format",
                messages.text(MsgId::CliHelpArgBenchFormat),
                messages,
            );
            set_arg_help(
                command,
                "output",
                messages.text(MsgId::CliHelpArgBenchOutput),
                messages,
            );
            set_arg_help(
                command,
                "baseline",
                messages.text(MsgId::CliHelpArgBenchBaseline),
                messages,
            );
            set_arg_help(
                command,
                "samples",
                messages.text(MsgId::CliHelpArgBenchSamples),
                messages,
            );
        }
        _ => {}
    }
    assign_headings(command, messages);
}

/// A noun group lists its verbs under the localised heading. Clap's built-in `help` subcommand is
/// disabled because it is English-only; `--help` on the group and on each verb remain.
fn localise_group(command: &mut clap::Command, messages: &Messages, about: MsgId, path: &str) {
    set_about(command, messages.text(about));
    *command = std::mem::take(command)
        .subcommand_help_heading(messages.text(MsgId::CliHelpCommandsHeading))
        .disable_help_subcommand(true)
        .subcommand_required(true)
        .arg_required_else_help(true);
    for subcommand in command.get_subcommands_mut() {
        localise(subcommand, messages, path);
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

fn set_positional_arg_help(
    command: &mut clap::Command,
    id: &'static str,
    help: String,
    messages: &Messages,
) {
    *command = std::mem::take(command).mut_arg(id, |arg| {
        arg.help(help)
            .help_heading(messages.text(MsgId::CliHelpArgumentsHeading))
    });
}

fn set_about(command: &mut clap::Command, about: String) {
    *command = std::mem::take(command).about(about);
}
