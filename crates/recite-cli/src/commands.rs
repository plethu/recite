use std::io::Write;

use crate::args::{Command, CompileArgs, ExplainArgs, ExtractArgs, RuntimeArgs, TraceArgs};
use crate::diagnostics::{Subject, report_diagnostics, report_success};
use crate::dialogue_locale::LoadedDialoguePreview;
use crate::error::CliError;
use crate::fs::{
    collect_input_files, compile_options, load_optional_schema, read_compile_inputs_for_output,
    read_compile_inputs_from_files, reject_output_input_alias, write_staged,
};
use crate::i18n::{Messages, MsgId};
use crate::play::run_play_command;
use crate::runtime_fixture::{
    RuntimeFixtureOptions, dialogue_preview_from_fixture, execute_runtime_fixture,
    load_compiled_asset, load_runtime_fixture,
};
use crate::watch::run_watch_command;
use recite_compiler::{
    compile::{compile_inputs_to_messagepack, compile_inputs_to_messagepack_with_schema},
    pot::{extract_pot, extract_pot_with_schema},
};
use recite_core::{DiagnosticCode, explain_diagnostic_code, suggest_diagnostic_code};

mod bench;
mod catalog;
mod id;
mod project;
mod project_files;
mod recording;
mod schema;
mod validate;
use bench::bench_command;

pub(crate) fn run_command(
    command: Command,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    match command {
        Command::Validate(args) => validate::run(args, stderr, messages),
        Command::Compile(args) => compile_command(args, stderr, messages),
        Command::Extract(args) => extract_command(args, stdout, stderr, messages),
        Command::Catalog(command) => catalog::run(command, stdout, stderr, messages),
        Command::Recording(command) => recording::run(command, stdout, stderr, messages),
        Command::Id(command) => id::run(command, stderr, messages),
        Command::Schema(command) => schema::run(command, stdout, stderr, messages),
        Command::Project(command) => project::run(command, stderr, messages),
        Command::Explain(args) => explain_command(args, stdout, messages),
        Command::Watch(args) => run_watch_command(args, stderr, messages),
        Command::Run(args) => runtime_command(args, RuntimeOutput::Run, stdout, messages),
        Command::Trace(args) => trace_command(args, stdout, messages),
        Command::Play(args) => run_play_command(args, stdout, stderr),
        Command::Bench(args) => bench_command(args, stdout),
        Command::Import(args) => crate::migration::run(args, stdout, stderr, messages),
    }
}

fn explain_command(
    args: ExplainArgs,
    stdout: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let code =
        DiagnosticCode::new(args.code.clone()).map_err(|_| CliError::DiagnosticCodeMalformed {
            suggestion: diagnostic_code_suggestion(&args.code),
            code: args.code.clone(),
        })?;
    let explanation =
        explain_diagnostic_code(&code).ok_or_else(|| CliError::DiagnosticCodeUnknown {
            suggestion: diagnostic_code_suggestion(&args.code),
            code: args.code.clone(),
        })?;

    writeln!(
        stdout,
        "{}",
        messages.format(
            MsgId::ExplainCode,
            [("code", explanation.code.as_str().to_owned())]
        )
    )?;
    writeln!(
        stdout,
        "{}",
        messages.format(
            MsgId::ExplainCategory,
            [("category", explanation.category.as_str().to_owned())]
        )
    )?;
    writeln!(
        stdout,
        "{}",
        messages.format(
            MsgId::ExplainMeaning,
            [("meaning", explanation.meaning.to_owned())]
        )
    )?;
    writeln!(stdout, "{}", messages.text(MsgId::ExplainCommonCauses))?;
    for cause in explanation.common_causes {
        writeln!(
            stdout,
            "{}",
            messages.format(MsgId::ExplainListItem, [("item", cause.to_string())])
        )?;
    }
    writeln!(stdout, "{}", messages.text(MsgId::ExplainHowToFix))?;
    for remediation in explanation.remediation {
        writeln!(
            stdout,
            "{}",
            messages.format(MsgId::ExplainListItem, [("item", remediation.to_string())])
        )?;
    }
    Ok(())
}

fn diagnostic_code_suggestion(input: &str) -> Option<String> {
    suggest_diagnostic_code(input).map(|explanation| explanation.code.as_str().to_owned())
}

fn compile_command(
    args: CompileArgs,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let input_files = collect_input_files(&args.paths)?;
    reject_output_input_alias(&args.output, &input_files)?;
    let file_count = input_files.len();
    let inputs = read_compile_inputs_for_output(&args.output, input_files)?;
    let schema = load_optional_schema(args.schema.as_deref(), stderr, messages)?;
    let options = compile_options(&args.output, schema.as_ref())?;
    let report = if let Some(schema) = &schema {
        compile_inputs_to_messagepack_with_schema(inputs, options, schema)?
    } else {
        compile_inputs_to_messagepack(inputs, options)?
    };

    let reported = report_diagnostics(stderr, messages, report.diagnostics.iter())?;
    let Some(messagepack) = report.messagepack else {
        return Err(CliError::Diagnostics);
    };

    write_staged(&args.output, &messagepack)?;
    if reported == 0 {
        report_success(stderr, messages, Subject::Files(file_count))?;
    }
    Ok(())
}

fn extract_command(
    args: ExtractArgs,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let input_files = collect_input_files(&args.paths)?;
    if let Some(output) = &args.output {
        reject_output_input_alias(output, &input_files)?;
    }
    let inputs = read_compile_inputs_from_files(input_files)?;
    let schema = load_optional_schema(args.schema.as_deref(), stderr, messages)?;
    let report = if let Some(schema) = &schema {
        extract_pot_with_schema(inputs, schema)
    } else {
        extract_pot(inputs)
    };

    report_diagnostics(stderr, messages, report.diagnostics.iter())?;
    let Some(catalog) = report.catalog else {
        return Err(CliError::Diagnostics);
    };

    let pot = catalog.to_pot_string();
    if let Some(output) = args.output {
        write_staged(&output, pot.as_bytes())?;
    } else {
        stdout.write_all(pot.as_bytes())?;
    }
    Ok(())
}

fn runtime_command(
    args: RuntimeArgs,
    output: RuntimeOutput,
    stdout: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    runtime_command_with_options(
        args,
        output,
        RuntimeFixtureOptions::default(),
        stdout,
        messages,
    )
}

fn trace_command(
    args: TraceArgs,
    stdout: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    runtime_command_with_options(
        args.runtime,
        RuntimeOutput::Trace,
        RuntimeFixtureOptions {
            metrics: args.metrics,
        },
        stdout,
        messages,
    )
}

fn runtime_command_with_options(
    args: RuntimeArgs,
    output: RuntimeOutput,
    options: RuntimeFixtureOptions,
    stdout: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let asset = load_compiled_asset(&args.asset)?;
    let fixture = load_runtime_fixture(&args.fixture)?;
    let dialogue_preview = dialogue_preview_from_fixture(&fixture)?
        .map(LoadedDialoguePreview::load)
        .transpose()?;
    let execution = execute_runtime_fixture(
        &asset,
        &args.block,
        &fixture,
        dialogue_preview
            .as_ref()
            .map(LoadedDialoguePreview::traversal_preview),
        dialogue_preview
            .as_ref()
            .map(LoadedDialoguePreview::locale_fallbacks),
        options,
        messages,
    )?;

    match output {
        RuntimeOutput::Run => {
            for line in execution.run_lines {
                writeln!(stdout, "{line}")?;
            }
        }
        RuntimeOutput::Trace => {
            serde_json::to_writer_pretty(&mut *stdout, &execution.trace)
                .map_err(CliError::TraceJson)?;
            writeln!(stdout)?;
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeOutput {
    Run,
    Trace,
}
