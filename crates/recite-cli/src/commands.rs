use std::io::Write;

use crate::args::{Command, CompileArgs, ExplainArgs, ExtractArgs, RuntimeArgs, TraceArgs};
use crate::diagnostics::{report_diagnostics, report_targeted_diagnostics};
use crate::dialogue_locale::LoadedDialoguePreview;
use crate::error::CliError;
use crate::fs::{
    check_fresh, collect_input_files, compile_options, load_optional_schema, load_schema,
    read_compile_inputs_for_output, read_compile_inputs_from_files, reject_output_input_alias,
    validate_inputs, validate_project, write_staged,
};
use crate::i18n::{Messages, MsgId};
use crate::play::run_play_command;
use crate::runtime_fixture::{
    RuntimeFixtureOptions, dialogue_preview_from_fixture, execute_runtime_fixture,
    load_compiled_asset, load_runtime_fixture,
};
use crate::watch::run_watch_command;
use recite_compiler::{
    compile::{compile_inputs, compile_inputs_with_schema},
    pot::{extract_pot, extract_pot_with_schema},
};
use recite_core::{
    DiagnosticCategory, DiagnosticCode, explain_diagnostic_code, suggest_diagnostic_code,
};

mod bench;
mod project;
use bench::bench_command;
use project::project_check;

pub(crate) fn run_command(
    command: Command,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    match command {
        Command::Validate(args) => {
            let diagnostics = validate_inputs(&args.paths, None)?.into_all();
            report_diagnostics(stderr, messages, diagnostics.iter())?;
            diagnostics
                .is_empty()
                .then_some(())
                .ok_or(CliError::Diagnostics)
        }
        Command::Compile(args) => compile_command(args, stderr, messages),
        Command::Extract(args) => extract_command(args, stdout, stderr, messages),
        Command::CheckIds(args) => {
            let diagnostics = validate_inputs(&args.paths, None)?;
            report_targeted_diagnostics(stderr, messages, diagnostics, |diagnostic| {
                diagnostic.code.category() == DiagnosticCategory::Identifier
            })
        }
        Command::CheckMarkup(args) => {
            let schema = load_optional_schema(args.schema.as_deref(), stderr, messages)?;
            let diagnostics = validate_inputs(&args.paths, schema.as_ref())?;
            report_targeted_diagnostics(stderr, messages, diagnostics, |diagnostic| {
                diagnostic.code.category() == DiagnosticCategory::Markup
            })
        }
        Command::CheckMetadata(args) => {
            let schema = load_schema(&args.schema)?;
            if !schema.diagnostics.is_empty() {
                report_diagnostics(stderr, messages, schema.diagnostics.iter())?;
                return Err(CliError::Diagnostics);
            }

            let diagnostics = validate_inputs(&args.paths, schema.schema.as_ref())?;
            report_targeted_diagnostics(stderr, messages, diagnostics, |diagnostic| {
                diagnostic.code.category() == DiagnosticCategory::Metadata
            })
        }
        Command::ValidateProject(args) => {
            project_check(args.project_root, validate_project, stderr, messages)
        }
        Command::CheckFresh(args) => {
            project_check(args.project_root, check_fresh, stderr, messages)
        }
        Command::CheckSchemaProducerFreshness(args) => {
            crate::schema_freshness::check(args, stdout, stderr, messages)
        }
        Command::InspectSchema(args) => {
            crate::schema_inspection::run(args, stdout, stderr, messages)
        }
        Command::ExportSchema(args) => crate::schema_export::run(args, stdout, stderr, messages),
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
    let inputs = read_compile_inputs_for_output(&args.output, input_files)?;
    let schema = load_optional_schema(args.schema.as_deref(), stderr, messages)?;
    let options = compile_options(&args.output, schema.as_ref())?;
    let report = if let Some(schema) = &schema {
        compile_inputs_with_schema(inputs, options, schema)?
    } else {
        compile_inputs(inputs, options)?
    };

    report_diagnostics(stderr, messages, report.diagnostics.iter())?;
    let Some(asset) = report.asset else {
        return Err(CliError::Diagnostics);
    };

    write_staged(&args.output, &asset.messagepack)?;
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
