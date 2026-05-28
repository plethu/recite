use std::io::Write;

use recite_compiler::{
    compile_inputs, compile_inputs_with_schema, extract_pot, extract_pot_with_schema,
};

use crate::args::{Command, CompileArgs, ExtractArgs, RuntimeArgs};
use crate::diagnostics::{report_diagnostics, report_targeted_diagnostics};
use crate::dialogue_locale::LoadedDialoguePreview;
use crate::error::CliError;
use crate::fs::{
    collect_input_files, compile_options, load_optional_schema, load_schema,
    read_compile_inputs_for_output, read_compile_inputs_from_files, reject_output_input_alias,
    validate_inputs, validate_project, write_staged,
};
use crate::play::run_play_command;
use crate::runtime_fixture::{
    dialogue_preview_from_fixture, execute_runtime_fixture, load_compiled_asset,
    load_runtime_fixture,
};
use crate::watch::run_watch_command;

pub(crate) fn run_command(
    command: Command,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<(), CliError> {
    match command {
        Command::Validate(args) => {
            let diagnostics = validate_inputs(&args.paths, None)?.into_all();
            report_diagnostics(stderr, diagnostics.iter())?;
            diagnostics
                .is_empty()
                .then_some(())
                .ok_or(CliError::Diagnostics)
        }
        Command::Compile(args) => compile_command(args, stderr),
        Command::Extract(args) => extract_command(args, stdout, stderr),
        Command::CheckIds(args) => {
            let diagnostics = validate_inputs(&args.paths, None)?;
            report_targeted_diagnostics(stderr, diagnostics, |diagnostic| {
                diagnostic.code.as_str().starts_with("RECITE_ID")
            })
        }
        Command::CheckMarkup(args) => {
            let schema = load_optional_schema(args.schema.as_deref(), stderr)?;
            let diagnostics = validate_inputs(&args.paths, schema.as_ref())?;
            report_targeted_diagnostics(stderr, diagnostics, |diagnostic| {
                matches!(
                    diagnostic.code.as_str(),
                    "RECITE_VALIDATE022"
                        | "RECITE_VALIDATE023"
                        | "RECITE_VALIDATE024"
                        | "RECITE_VALIDATE025"
                )
            })
        }
        Command::CheckMetadata(args) => {
            let schema = load_schema(&args.schema)?;
            if !schema.diagnostics.is_empty() {
                report_diagnostics(stderr, schema.diagnostics.iter())?;
                return Err(CliError::Diagnostics);
            }

            let diagnostics = validate_inputs(&args.paths, schema.schema.as_ref())?;
            report_targeted_diagnostics(stderr, diagnostics, |diagnostic| {
                matches!(
                    diagnostic.code.as_str(),
                    "RECITE_VALIDATE026"
                        | "RECITE_VALIDATE027"
                        | "RECITE_VALIDATE028"
                        | "RECITE_VALIDATE029"
                        | "RECITE_VALIDATE030"
                )
            })
        }
        Command::ValidateProject(args) | Command::CheckFresh(args) => {
            let diagnostics = validate_project(args.project_root)?;
            report_diagnostics(stderr, diagnostics.iter())?;
            diagnostics
                .is_empty()
                .then_some(())
                .ok_or(CliError::Diagnostics)
        }
        Command::Watch(args) => run_watch_command(args, stderr),
        Command::Run(args) => runtime_command(args, RuntimeOutput::Run, stdout),
        Command::Trace(args) => runtime_command(args, RuntimeOutput::Trace, stdout),
        Command::Play(args) => run_play_command(args, stdout, stderr),
    }
}

fn compile_command(args: CompileArgs, stderr: &mut dyn Write) -> Result<(), CliError> {
    let input_files = collect_input_files(&args.paths)?;
    reject_output_input_alias(&args.output, &input_files)?;
    let inputs = read_compile_inputs_for_output(&args.output, input_files)?;
    let schema = load_optional_schema(args.schema.as_deref(), stderr)?;
    let options = compile_options(&args.output, schema.as_ref())?;
    let report = if let Some(schema) = &schema {
        compile_inputs_with_schema(inputs, options, schema)?
    } else {
        compile_inputs(inputs, options)?
    };

    report_diagnostics(stderr, report.diagnostics.iter())?;
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
) -> Result<(), CliError> {
    let input_files = collect_input_files(&args.paths)?;
    if let Some(output) = &args.output {
        reject_output_input_alias(output, &input_files)?;
    }
    let inputs = read_compile_inputs_from_files(input_files)?;
    let schema = load_optional_schema(args.schema.as_deref(), stderr)?;
    let report = if let Some(schema) = &schema {
        extract_pot_with_schema(inputs, schema)
    } else {
        extract_pot(inputs)
    };

    report_diagnostics(stderr, report.diagnostics.iter())?;
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
