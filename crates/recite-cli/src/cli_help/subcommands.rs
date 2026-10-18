//! Localised command and argument metadata; clap owns parsing.
use super::{assign_headings, help_arg, help_template};
use crate::i18n::{Messages, MsgId};

pub(super) fn localise(command: &mut clap::Command, messages: &Messages) {
    *command = std::mem::take(command)
        .help_template(help_template(messages))
        .next_help_heading(messages.text(MsgId::CliHelpOptionsHeading))
        .disable_help_flag(true)
        .arg(help_arg(messages));

    match command.get_name() {
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
        "inspect-schema" => {
            set_about(command, messages.text(MsgId::CliHelpCommandInspectSchema));
            set_positional_arg_help(
                command,
                "schema",
                messages.text(MsgId::CliHelpArgSchemaInspection),
                messages,
            );
        }
        "export-schema" => {
            set_about(command, messages.text(MsgId::CliHelpCommandExportSchema));
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
        "check-schema-producer-freshness" => {
            set_about(
                command,
                messages.text(MsgId::CliHelpCommandCheckSchemaProducerFreshness),
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
