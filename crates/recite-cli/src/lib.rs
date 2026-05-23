use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use recite_compiler::{
    CompileInput, CompileOptions, compile_inputs, compile_inputs_with_schema, extract_pot,
    extract_pot_with_schema, validate_source_files, validate_source_files_with_schema,
};
use recite_core::{
    COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0, ChoiceId,
    CompiledAssetDecodeError, CompiledAssetId, CompiledDialogue, CompiledStatementKind,
    CompilerVersion, Diagnostic, DiagnosticCode, DiagnosticSeverity, MetadataEntry,
    ProjectFreshnessInput, ProjectManifest, ProjectSchema, ScalarValue, SchemaFingerprint,
    SourceMapId, SourceSpan, Value, decode_compiled_dialogue_messagepack, load_schema_manifest_str,
    project::{
        MALFORMED_COMPILED_ASSET, MISSING_COMPILED_ASSET, STALE_COMPILER_COMPATIBILITY,
        project_scene_key_span, validate_project_freshness, validate_project_manifest,
    },
};
use recite_parser::parse;
use recite_runtime::{
    ConditionArgument, ConditionEvaluationError, ConditionQuery, DialogueChoice, DialogueContext,
    DialogueEffectArgument, DialogueEffectMode, DialogueEffectRequest, DialogueEvent, DialogueLine,
    EffectAck, acknowledge_effect, choose as runtime_choose, next as runtime_next, start_scene,
};
use serde::{Deserialize, Serialize};

const SUCCESS: ExitCode = ExitCode::SUCCESS;
const PROJECT_MANIFEST_FILE: &str = "recite.project.toml";
#[derive(Debug, Parser)]
#[command(
    name = "recite",
    version,
    about = "Recite dialogue compiler and validation CLI."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate dialogue source without writing compiled output.
    Validate(InputArgs),
    /// Compile dialogue source to a MessagePack .recitec asset.
    Compile(CompileArgs),
    /// Extract gettext POT entries.
    Extract(ExtractArgs),
    /// Report stable line and choice ID diagnostics.
    #[command(name = "check-ids")]
    CheckIds(InputArgs),
    /// Validate inline markup, optionally against a schema manifest.
    #[command(name = "check-markup")]
    CheckMarkup(SchemaInputArgs),
    /// Validate metadata against a schema manifest.
    #[command(name = "check-metadata")]
    CheckMetadata(RequiredSchemaInputArgs),
    /// Validate recite.project.toml and referenced compiled assets.
    #[command(name = "validate-project")]
    ValidateProject(ProjectRootArgs),
    /// Check whether project compiled assets are fresh.
    #[command(name = "check-fresh")]
    CheckFresh(ProjectRootArgs),
    /// Run a compiled asset headlessly with fixture data.
    Run(RuntimeArgs),
    /// Emit deterministic JSON for a headless fixture run.
    Trace(RuntimeArgs),
}

#[derive(Debug, Args)]
struct InputArgs {
    /// One or more .recite files, or directories containing .recite files.
    #[arg(required = true)]
    paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
struct SchemaInputArgs {
    /// Generated schema manifest JSON.
    #[arg(long)]
    schema: Option<PathBuf>,
    /// One or more .recite files, or directories containing .recite files.
    #[arg(required = true)]
    paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
struct RequiredSchemaInputArgs {
    /// Generated schema manifest JSON.
    #[arg(long)]
    schema: PathBuf,
    /// One or more .recite files, or directories containing .recite files.
    #[arg(required = true)]
    paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
struct ProjectRootArgs {
    /// Project root containing recite.project.toml.
    project_root: PathBuf,
}

#[derive(Debug, Args)]
struct CompileArgs {
    /// Write MessagePack compiled asset bytes to this path.
    #[arg(short, long)]
    output: PathBuf,
    /// Generated schema manifest JSON.
    #[arg(long)]
    schema: Option<PathBuf>,
    /// One or more .recite files, or directories containing .recite files.
    #[arg(required = true)]
    paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
struct ExtractArgs {
    /// Write POT output to this path instead of stdout.
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Generated schema manifest JSON.
    #[arg(long)]
    schema: Option<PathBuf>,
    /// One or more .recite files, or directories containing .recite files.
    #[arg(required = true)]
    paths: Vec<PathBuf>,
}

#[derive(Debug, Args)]
struct RuntimeArgs {
    /// MessagePack .recitec asset to run.
    asset: PathBuf,
    /// Block ID to start from.
    #[arg(long)]
    block: String,
    /// TOML fixture with conditions, choices, and effect options.
    #[arg(long)]
    fixture: PathBuf,
}

pub fn run(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    let cli = Cli::parse_from(args);
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();

    match run_command(cli.command, &mut stdout, &mut stderr) {
        Ok(()) => SUCCESS,
        Err(CliError::Diagnostics) => ExitCode::from(1),
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            ExitCode::from(1)
        }
    }
}

fn run_command(
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
        Command::Run(args) => runtime_command(args, RuntimeOutput::Run, stdout),
        Command::Trace(args) => runtime_command(args, RuntimeOutput::Trace, stdout),
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
    let execution = execute_runtime_fixture(&asset, &args.block, &fixture)?;

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
enum RuntimeOutput {
    Run,
    Trace,
}

fn load_compiled_asset(path: &Path) -> Result<CompiledDialogue, CliError> {
    let bytes = fs::read(path).map_err(|source| CliError::Read {
        path: path.to_owned(),
        source,
    })?;
    decode_compiled_dialogue_messagepack(&bytes).map_err(|source| CliError::DecodeAsset {
        path: path.to_owned(),
        source,
    })
}

fn load_runtime_fixture(path: &Path) -> Result<RuntimeFixture, CliError> {
    let source = fs::read_to_string(path).map_err(|source| CliError::Read {
        path: path.to_owned(),
        source,
    })?;
    toml::from_str(&source).map_err(|source| CliError::FixtureToml {
        path: path.to_owned(),
        source,
    })
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeFixture {
    #[serde(default)]
    conditions: BTreeMap<String, bool>,
    #[serde(default)]
    choices: BTreeMap<String, FixtureChoice>,
    #[serde(default)]
    effects: FixtureEffects,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum FixtureChoice {
    Id(String),
    Index(usize),
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureEffects {
    #[serde(default)]
    auto_ack_blocking: bool,
}

struct RuntimeExecution {
    run_lines: Vec<String>,
    trace: TraceDocument,
}

fn execute_runtime_fixture(
    asset: &CompiledDialogue,
    block: &str,
    fixture: &RuntimeFixture,
) -> Result<RuntimeExecution, CliError> {
    let prompt_catalog = PromptCatalog::new(asset)?;
    let context = FixtureContext::new(&fixture.conditions);
    let mut session = start_scene(asset, Some(block))?;
    let mut trace_events = Vec::new();
    let mut run_lines = Vec::new();
    let mut pending_event = None;
    let final_deferred_effects: Vec<TraceEffect>;

    loop {
        let event = match pending_event.take() {
            Some(event) => event,
            None => {
                let event = runtime_next(asset, &mut session, &context)?;
                record_conditions(&context, &mut run_lines, &mut trace_events);
                event
            }
        };

        match event {
            DialogueEvent::Line(line) => {
                run_lines.push(format!("line {}: {}", line.id.as_str(), line.text));
                trace_events.push(TraceEvent::Line {
                    line: trace_line(&line),
                });
            }
            DialogueEvent::Prompt { line, choices } => {
                let prompt = prompt_catalog.identify(line.as_ref(), &choices)?;
                write_prompt_run_lines(&mut run_lines, &prompt, line.as_ref(), &choices);
                trace_events.push(TraceEvent::Prompt {
                    prompt: trace_prompt(&prompt, line.as_ref(), &choices),
                });

                let choice_id = select_fixture_choice(fixture, &prompt, &choices)?;
                run_lines.push(format!("selected choice {}", choice_id.as_str()));
                trace_events.push(TraceEvent::ChoiceSelected {
                    prompt: trace_prompt_identity(&prompt),
                    choice: choice_id.as_str().to_owned(),
                });

                let event = runtime_choose(asset, &mut session, choice_id, &context)?;
                record_conditions(&context, &mut run_lines, &mut trace_events);
                pending_event = Some(event);
            }
            DialogueEvent::Effect(effect) => {
                run_lines.push(format!(
                    "effect {} {} {}",
                    effect.mode,
                    effect.function,
                    format_effect_arguments(&effect.args)
                ));
                trace_events.push(TraceEvent::Effect {
                    effect: trace_effect(&effect),
                });

                if effect.mode == DialogueEffectMode::Blocking {
                    if !fixture.effects.auto_ack_blocking {
                        return Err(CliError::BlockingEffectNeedsAcknowledgement {
                            effect: effect.id.as_str().to_owned(),
                        });
                    }

                    acknowledge_effect(&mut session, effect.id.clone(), EffectAck::Completed)?;
                    run_lines.push(format!(
                        "acknowledged effect {} completed",
                        effect.id.as_str()
                    ));
                    trace_events.push(TraceEvent::Acknowledgement {
                        effect_id: effect.id.as_str().to_owned(),
                        result: "completed",
                    });
                }
            }
            DialogueEvent::End { deferred_effects } => {
                run_lines.push("end".to_owned());
                if !deferred_effects.is_empty() {
                    run_lines.push("deferred effects:".to_owned());
                    for effect in &deferred_effects {
                        run_lines.push(format!(
                            "  {} {}",
                            effect.function,
                            format_effect_arguments(&effect.args)
                        ));
                    }
                }

                final_deferred_effects = deferred_effects.iter().map(trace_effect).collect();
                trace_events.push(TraceEvent::End {
                    deferred_effects: final_deferred_effects.clone(),
                });
                break;
            }
        }
    }

    Ok(RuntimeExecution {
        run_lines,
        trace: TraceDocument {
            asset_id: asset.header.asset_id.as_str().to_owned(),
            block: block.to_owned(),
            events: trace_events,
            final_deferred_effects,
        },
    })
}

fn record_conditions(
    context: &FixtureContext<'_>,
    run_lines: &mut Vec<String>,
    trace_events: &mut Vec<TraceEvent>,
) {
    for condition in context.take_records() {
        run_lines.push(format!(
            "condition {} = {}",
            condition.query, condition.result
        ));
        trace_events.push(TraceEvent::Condition { condition });
    }
}

struct FixtureContext<'a> {
    conditions: &'a BTreeMap<String, bool>,
    records: RefCell<Vec<TraceCondition>>,
}

impl<'a> FixtureContext<'a> {
    fn new(conditions: &'a BTreeMap<String, bool>) -> Self {
        Self {
            conditions,
            records: RefCell::new(Vec::new()),
        }
    }

    fn take_records(&self) -> Vec<TraceCondition> {
        self.records.take()
    }
}

impl DialogueContext for FixtureContext<'_> {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<bool, ConditionEvaluationError> {
        let arguments = query
            .arguments()
            .into_iter()
            .map(trace_condition_argument)
            .collect::<Vec<_>>();
        let query_text = condition_query_text(query.function(), &arguments);
        let Some(result) = self.conditions.get(&query_text).copied() else {
            return Err(ConditionEvaluationError::new(format!(
                "fixture is missing condition `{query_text}`"
            )));
        };

        self.records.borrow_mut().push(TraceCondition {
            query: query_text,
            function: query.function().to_owned(),
            arguments,
            result,
        });
        Ok(result)
    }
}

#[derive(Clone, Debug)]
struct PromptIdentity {
    block: String,
    line: Option<String>,
    choice_ids: Vec<String>,
    fixture_keys: Vec<String>,
}

struct PromptCatalog {
    prompts: Vec<PromptIdentity>,
}

impl PromptCatalog {
    fn new(asset: &CompiledDialogue) -> Result<Self, CliError> {
        let mut block_prompt_counts = BTreeMap::<String, usize>::new();
        let mut prompt_rows = Vec::<(String, Option<String>, Vec<String>)>::new();

        for block in &asset.blocks {
            let block_id = block.id.as_str().to_owned();
            let statement_start = block.statements.start.as_u32();
            let statement_end = statement_start + block.statements.len;

            for statement_index in statement_start..statement_end {
                let statement =
                    asset
                        .statements
                        .get(statement_index as usize)
                        .ok_or_else(|| CliError::MalformedCompiledAsset {
                            reason: format!("statement index {statement_index} is out of bounds"),
                        })?;
                let CompiledStatementKind::Prompt { line, choices } = &statement.kind else {
                    continue;
                };

                let line = line
                    .map(|line| {
                        asset
                            .lines
                            .get(line.as_u32() as usize)
                            .map(|line| line.id.as_str().to_owned())
                            .ok_or_else(|| CliError::MalformedCompiledAsset {
                                reason: format!("line index {} is out of bounds", line.as_u32()),
                            })
                    })
                    .transpose()?;
                let choice_start = choices.start.as_u32();
                let choice_end = choice_start + choices.len;
                let mut choice_ids = Vec::new();
                for choice_index in choice_start..choice_end {
                    let choice = asset.choices.get(choice_index as usize).ok_or_else(|| {
                        CliError::MalformedCompiledAsset {
                            reason: format!("choice index {choice_index} is out of bounds"),
                        }
                    })?;
                    choice_ids.push(choice.id.as_str().to_owned());
                }

                *block_prompt_counts.entry(block_id.clone()).or_default() += 1;
                prompt_rows.push((block_id.clone(), line, choice_ids));
            }
        }

        let prompts = prompt_rows
            .into_iter()
            .map(|(block, line, choice_ids)| {
                let mut fixture_keys = Vec::new();
                if let Some(line) = &line {
                    fixture_keys.push(line.clone());
                }
                if block_prompt_counts.get(&block) == Some(&1) {
                    fixture_keys.push(block.clone());
                }

                PromptIdentity {
                    block,
                    line,
                    choice_ids,
                    fixture_keys,
                }
            })
            .collect();

        Ok(Self { prompts })
    }

    fn identify(
        &self,
        line: Option<&DialogueLine>,
        choices: &[DialogueChoice],
    ) -> Result<PromptIdentity, CliError> {
        let line_id = line.map(|line| line.id.as_str());
        let choice_ids = choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>();

        self.prompts
            .iter()
            .find(|prompt| {
                prompt.line.as_deref() == line_id
                    && prompt
                        .choice_ids
                        .iter()
                        .map(String::as_str)
                        .eq(choice_ids.iter().copied())
            })
            .cloned()
            .ok_or_else(|| CliError::UnknownPrompt {
                line: line_id.map(str::to_owned),
                choices: choice_ids.into_iter().map(str::to_owned).collect(),
            })
    }
}

fn select_fixture_choice(
    fixture: &RuntimeFixture,
    prompt: &PromptIdentity,
    choices: &[DialogueChoice],
) -> Result<ChoiceId, CliError> {
    let selection = prompt
        .fixture_keys
        .iter()
        .find_map(|key| fixture.choices.get(key))
        .ok_or_else(|| CliError::MissingFixtureChoice {
            prompt_keys: prompt.fixture_keys.clone(),
        })?;

    match selection {
        FixtureChoice::Id(choice_id) => {
            let choice = ChoiceId::new(choice_id.clone())?;
            if !choices.iter().any(|candidate| candidate.id == choice) {
                return Err(CliError::FixtureChoiceNotInPrompt {
                    choice: choice_id.clone(),
                    prompt_keys: prompt.fixture_keys.clone(),
                });
            }
            Ok(choice)
        }
        FixtureChoice::Index(index) => {
            if *index == 0 || *index > choices.len() {
                return Err(CliError::FixtureChoiceIndexOutOfRange {
                    index: *index,
                    choice_count: choices.len(),
                    prompt_keys: prompt.fixture_keys.clone(),
                });
            }

            Ok(choices[*index - 1].id.clone())
        }
    }
}

fn write_prompt_run_lines(
    run_lines: &mut Vec<String>,
    prompt: &PromptIdentity,
    line: Option<&DialogueLine>,
    choices: &[DialogueChoice],
) {
    match line {
        Some(line) => run_lines.push(format!("prompt {}: {}", line.id.as_str(), line.text)),
        None => run_lines.push(format!("prompt {}", prompt.fixture_keys.join("|"))),
    }

    for (index, choice) in choices.iter().enumerate() {
        let availability = if choice.is_available {
            ""
        } else {
            " (unavailable)"
        };
        run_lines.push(format!(
            "  [{}] {}: {}{}",
            index + 1,
            choice.id.as_str(),
            choice.text,
            availability
        ));
    }
}

fn trace_prompt(
    prompt: &PromptIdentity,
    line: Option<&DialogueLine>,
    choices: &[DialogueChoice],
) -> TracePrompt {
    TracePrompt {
        identity: trace_prompt_identity(prompt),
        line: line.map(trace_line),
        choices: choices.iter().map(trace_choice).collect(),
    }
}

fn trace_prompt_identity(prompt: &PromptIdentity) -> TracePromptIdentity {
    TracePromptIdentity {
        block: prompt.block.clone(),
        line: prompt.line.clone(),
        fixture_keys: prompt.fixture_keys.clone(),
    }
}

fn trace_line(line: &DialogueLine) -> TraceLine {
    TraceLine {
        id: line.id.as_str().to_owned(),
        source_text: line.source_text.clone(),
        text: line.text.clone(),
        speaker: line
            .speaker
            .as_ref()
            .map(|speaker| speaker.as_str().to_owned()),
        metadata: line.metadata.iter().map(trace_metadata).collect(),
    }
}

fn trace_choice(choice: &DialogueChoice) -> TraceChoice {
    TraceChoice {
        id: choice.id.as_str().to_owned(),
        source_text: choice.source_text.clone(),
        text: choice.text.clone(),
        metadata: choice.metadata.iter().map(trace_metadata).collect(),
        is_available: choice.is_available,
        unavailable_reason: choice.unavailable_reason.clone(),
    }
}

fn trace_metadata(metadata: &MetadataEntry) -> TraceMetadata {
    TraceMetadata {
        key: metadata.key.clone(),
        value: trace_value(&metadata.value),
    }
}

fn trace_value(value: &Value) -> TraceValue {
    match value {
        Value::Scalar(value) => TraceValue::Scalar(trace_scalar(value)),
        Value::Array(values) => TraceValue::Array(values.iter().map(trace_scalar).collect()),
    }
}

fn trace_scalar(value: &ScalarValue) -> TraceScalar {
    match value {
        ScalarValue::String(value) => TraceScalar::String(value.clone()),
        ScalarValue::Integer(value) => TraceScalar::Integer(*value),
        ScalarValue::Float(value) => TraceScalar::Float(*value),
        ScalarValue::Boolean(value) => TraceScalar::Boolean(*value),
    }
}

fn trace_effect(effect: &DialogueEffectRequest) -> TraceEffect {
    TraceEffect {
        id: effect.id.as_str().to_owned(),
        mode: effect_mode_name(effect.mode),
        function: effect.function.clone(),
        args: effect.args.iter().map(trace_effect_argument).collect(),
        source_span: trace_source_span(&effect.source_span),
    }
}

fn trace_source_span(span: &SourceSpan) -> TraceSourceSpan {
    TraceSourceSpan {
        file: span.file.clone(),
        start_line: span.start.line(),
        start_column: span.start.column(),
        end_line: span.end.map(|end| end.line()),
        end_column: span.end.map(|end| end.column()),
    }
}

fn trace_condition_argument(argument: ConditionArgument<'_>) -> TraceScalar {
    match argument {
        ConditionArgument::Identifier(value) => TraceScalar::Identifier(value.to_owned()),
        ConditionArgument::String(value) => TraceScalar::String(value.to_owned()),
        ConditionArgument::Integer(value) => TraceScalar::Integer(value),
        ConditionArgument::Float(value) => TraceScalar::Float(value),
        ConditionArgument::Boolean(value) => TraceScalar::Boolean(value),
    }
}

fn trace_effect_argument(argument: &DialogueEffectArgument) -> TraceScalar {
    match argument {
        DialogueEffectArgument::Identifier(value) => TraceScalar::Identifier(value.clone()),
        DialogueEffectArgument::String(value) => TraceScalar::String(value.clone()),
        DialogueEffectArgument::Integer(value) => TraceScalar::Integer(*value),
        DialogueEffectArgument::Float(value) => TraceScalar::Float(*value),
        DialogueEffectArgument::Boolean(value) => TraceScalar::Boolean(*value),
    }
}

fn condition_query_text(function: &str, arguments: &[TraceScalar]) -> String {
    let arguments = arguments
        .iter()
        .map(format_condition_argument)
        .collect::<Vec<_>>()
        .join(", ");
    format!("{function}({arguments})")
}

fn format_condition_argument(argument: &TraceScalar) -> String {
    match argument {
        TraceScalar::Identifier(value) => value.clone(),
        TraceScalar::String(value) => {
            serde_json::to_string(value).expect("serializing a string cannot fail")
        }
        TraceScalar::Integer(value) => value.to_string(),
        TraceScalar::Float(value) => value.to_string(),
        TraceScalar::Boolean(value) => value.to_string(),
    }
}

fn format_effect_arguments(arguments: &[DialogueEffectArgument]) -> String {
    let arguments = arguments
        .iter()
        .map(|argument| format_condition_argument(&trace_effect_argument(argument)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("({arguments})")
}

fn effect_mode_name(mode: DialogueEffectMode) -> &'static str {
    match mode {
        DialogueEffectMode::Deferred => "deferred",
        DialogueEffectMode::Immediate => "immediate",
        DialogueEffectMode::Blocking => "blocking",
    }
}

#[derive(Serialize)]
struct TraceDocument {
    asset_id: String,
    block: String,
    events: Vec<TraceEvent>,
    final_deferred_effects: Vec<TraceEffect>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TraceEvent {
    Condition {
        condition: TraceCondition,
    },
    Line {
        line: TraceLine,
    },
    Prompt {
        prompt: TracePrompt,
    },
    ChoiceSelected {
        prompt: TracePromptIdentity,
        choice: String,
    },
    Effect {
        effect: TraceEffect,
    },
    Acknowledgement {
        effect_id: String,
        result: &'static str,
    },
    End {
        deferred_effects: Vec<TraceEffect>,
    },
}

#[derive(Clone, Debug, Serialize)]
struct TraceCondition {
    query: String,
    function: String,
    arguments: Vec<TraceScalar>,
    result: bool,
}

#[derive(Clone, Debug, Serialize)]
struct TracePrompt {
    identity: TracePromptIdentity,
    line: Option<TraceLine>,
    choices: Vec<TraceChoice>,
}

#[derive(Clone, Debug, Serialize)]
struct TracePromptIdentity {
    block: String,
    line: Option<String>,
    fixture_keys: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct TraceLine {
    id: String,
    source_text: String,
    text: String,
    speaker: Option<String>,
    metadata: Vec<TraceMetadata>,
}

#[derive(Clone, Debug, Serialize)]
struct TraceChoice {
    id: String,
    source_text: String,
    text: String,
    metadata: Vec<TraceMetadata>,
    is_available: bool,
    unavailable_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct TraceMetadata {
    key: String,
    value: TraceValue,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum TraceValue {
    Scalar(TraceScalar),
    Array(Vec<TraceScalar>),
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum TraceScalar {
    Identifier(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

#[derive(Clone, Debug, Serialize)]
struct TraceEffect {
    id: String,
    mode: &'static str,
    function: String,
    args: Vec<TraceScalar>,
    source_span: TraceSourceSpan,
}

#[derive(Clone, Debug, Serialize)]
struct TraceSourceSpan {
    file: String,
    start_line: u32,
    start_column: u32,
    end_line: Option<u32>,
    end_column: Option<u32>,
}

fn validate_project(project_root: PathBuf) -> Result<Vec<Diagnostic>, CliError> {
    let manifest_path = project_root.join(PROJECT_MANIFEST_FILE);
    let manifest_source = fs::read_to_string(&manifest_path).map_err(|source| CliError::Read {
        path: manifest_path.clone(),
        source,
    })?;
    let manifest_file = display_path(&manifest_path);
    let report = ProjectManifest::load_str(manifest_file.clone(), &manifest_source);
    let mut diagnostics = report.diagnostics;
    let Some(manifest) = report.manifest else {
        return Ok(diagnostics);
    };

    let loaded_schema = load_project_schema(&project_root, &manifest)?;
    diagnostics.extend(loaded_schema.diagnostics.iter().cloned());
    diagnostics.extend(validate_project_manifest(
        &manifest_file,
        &manifest_source,
        &manifest,
        loaded_schema.schema.as_ref(),
    ));

    let current_schema_fingerprint = match loaded_schema.schema.as_ref() {
        Some(schema) => Some(ProjectSchema::canonical_fingerprint(schema)),
        None if loaded_schema.diagnostics.is_empty() => Some(SchemaFingerprint::NoSchema),
        None => None,
    };

    for (scene_index, scene) in manifest.scenes.iter().enumerate() {
        let asset_path = resolve_project_path(&project_root, &scene.asset);
        if !asset_path.is_file() {
            diagnostics.push(project_diagnostic(
                MISSING_COMPILED_ASSET,
                format!(
                    "scene '{}' references missing compiled asset '{}'",
                    scene.id, scene.asset
                ),
                project_scene_key_span(&manifest_file, &manifest_source, scene_index, "asset"),
            ));
            continue;
        }

        let bytes = fs::read(&asset_path).map_err(|source| CliError::Read {
            path: asset_path.clone(),
            source,
        })?;
        let asset = match decode_compiled_dialogue_messagepack(&bytes) {
            Ok(asset) => asset,
            Err(CompiledAssetDecodeError::UnsupportedFormat {
                format_version,
                compiler_compatibility_version,
            }) if format_version == COMPILED_ASSET_FORMAT_VERSION_V0
                && compiler_compatibility_version != COMPILER_COMPATIBILITY_VERSION_V0 =>
            {
                diagnostics.push(project_diagnostic(
                    STALE_COMPILER_COMPATIBILITY,
                    format!(
                        "compiled asset '{}' uses compiler compatibility version {}, expected {}",
                        scene.asset,
                        compiler_compatibility_version,
                        COMPILER_COMPATIBILITY_VERSION_V0
                    ),
                    project_scene_key_span(&manifest_file, &manifest_source, scene_index, "asset"),
                ));
                continue;
            }
            Err(error) => {
                diagnostics.push(project_diagnostic(
                    MALFORMED_COMPILED_ASSET,
                    format!(
                        "scene '{}' references malformed compiled asset '{}': {error}",
                        scene.id, scene.asset
                    ),
                    project_scene_key_span(&manifest_file, &manifest_source, scene_index, "asset"),
                ));
                continue;
            }
        };

        let current_sources = read_project_sources(&project_root, &asset_path, &asset.sources);
        let current_source_map = current_sources
            .iter()
            .map(|(path, source)| (path.as_str(), source.as_deref()))
            .collect::<BTreeMap<_, _>>();
        diagnostics.extend(validate_project_freshness(
            &manifest_file,
            &manifest_source,
            ProjectFreshnessInput {
                scene_index,
                scene,
                asset: &asset,
                current_sources: current_source_map,
                current_schema_fingerprint: current_schema_fingerprint.clone(),
            },
        ));
    }

    Ok(diagnostics)
}

fn load_project_schema(
    project_root: &Path,
    manifest: &ProjectManifest,
) -> Result<LoadedSchema, CliError> {
    let Some(schema_path) = manifest.project.schema.as_deref() else {
        return Ok(LoadedSchema {
            schema: None,
            diagnostics: Vec::new(),
        });
    };

    load_schema(&resolve_project_path(project_root, schema_path))
}

fn read_project_sources(
    project_root: &Path,
    asset_path: &Path,
    sources: &[recite_core::CompiledSourceFile],
) -> Vec<(String, Option<String>)> {
    sources
        .iter()
        .map(|source| {
            let current_source = project_source_candidates(project_root, asset_path, &source.path)
                .into_iter()
                .find_map(|path| fs::read_to_string(path).ok());
            (source.path.clone(), current_source)
        })
        .collect()
}

fn project_source_candidates(
    project_root: &Path,
    asset_path: &Path,
    source_path: &str,
) -> Vec<PathBuf> {
    let source_path = Path::new(source_path);
    if source_path.is_absolute() {
        return vec![source_path.to_owned()];
    }

    let mut candidates = Vec::new();
    let mut ancestor = asset_path.parent();
    while let Some(directory) = ancestor {
        let candidate = directory.join(source_path);
        if !candidates.iter().any(|existing| existing == &candidate) {
            candidates.push(candidate);
        }

        if directory == project_root {
            break;
        }
        ancestor = directory.parent();
    }

    let project_candidate = project_root.join(source_path);
    if !candidates
        .iter()
        .any(|existing| existing == &project_candidate)
    {
        candidates.push(project_candidate);
    }

    candidates
}

fn validate_inputs(
    paths: &[PathBuf],
    schema: Option<&ProjectSchema>,
) -> Result<InputDiagnostics, CliError> {
    let inputs = read_compile_inputs(paths)?;
    let mut source_files = Vec::new();
    let mut parse_diagnostics = Vec::new();

    for input in inputs {
        let parse = parse(&input.path, &input.source);
        let lowered = parse.lower_source_file();
        parse_diagnostics.extend(lowered.diagnostics);
        source_files.push(lowered.source_file);
    }

    let validation_diagnostics = if parse_diagnostics.is_empty() {
        if let Some(schema) = schema {
            validate_source_files_with_schema(&source_files, schema)
        } else {
            validate_source_files(&source_files)
        }
        .diagnostics
    } else {
        Vec::new()
    };

    Ok(InputDiagnostics {
        parse_diagnostics,
        validation_diagnostics,
    })
}

fn read_compile_inputs(paths: &[PathBuf]) -> Result<Vec<CompileInput>, CliError> {
    read_compile_inputs_from_files(collect_input_files(paths)?)
}

fn collect_input_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>, CliError> {
    let mut files = Vec::new();
    for path in paths {
        collect_recite_files(path, &mut files)?;
    }
    files.sort();

    if files.is_empty() {
        return Err(CliError::NoInputs);
    }

    Ok(files)
}

fn read_compile_inputs_from_files(files: Vec<PathBuf>) -> Result<Vec<CompileInput>, CliError> {
    files
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path).map_err(|source| CliError::Read {
                path: path.clone(),
                source,
            })?;
            Ok(CompileInput::new(display_path(&path), source))
        })
        .collect()
}

fn read_compile_inputs_for_output(
    output: &Path,
    files: Vec<PathBuf>,
) -> Result<Vec<CompileInput>, CliError> {
    let project_root = compile_path_root(output, &files);
    files
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path).map_err(|source| CliError::Read {
                path: path.clone(),
                source,
            })?;
            let input_path = project_root
                .as_ref()
                .and_then(|root| project_relative_path(root, &path))
                .unwrap_or_else(|| display_path(&path));
            Ok(CompileInput::new(input_path, source))
        })
        .collect()
}

fn compile_path_root(output: &Path, files: &[PathBuf]) -> Option<PathBuf> {
    let output = canonical_output_path(output)?;
    let mut root = output.parent()?.to_owned();

    for file in files {
        let canonical = fs::canonicalize(file).ok()?;
        root = common_path_prefix(&root, &canonical)?;
    }

    (root.components().count() > 1).then_some(root)
}

fn project_relative_path(root: &Path, path: &Path) -> Option<String> {
    let canonical = fs::canonicalize(path).ok()?;
    canonical
        .strip_prefix(root)
        .ok()
        .map(display_path)
        .filter(|path| !path.is_empty())
}

fn common_path_prefix(left: &Path, right: &Path) -> Option<PathBuf> {
    let mut prefix = PathBuf::new();
    for (left_component, right_component) in left.components().zip(right.components()) {
        if left_component != right_component {
            break;
        }
        prefix.push(left_component.as_os_str());
    }

    (!prefix.as_os_str().is_empty()).then_some(prefix)
}

fn reject_output_input_alias(output: &Path, input_files: &[PathBuf]) -> Result<(), CliError> {
    let Some(output) = canonical_output_path(output) else {
        return Ok(());
    };

    for input in input_files {
        let Ok(input_canonical) = fs::canonicalize(input) else {
            continue;
        };
        if output == input_canonical {
            return Err(CliError::OutputOverwritesInput {
                output: output.clone(),
                input: input.clone(),
            });
        }
    }

    Ok(())
}

fn canonical_output_path(output: &Path) -> Option<PathBuf> {
    if let Ok(canonical) = fs::canonicalize(output) {
        return Some(canonical);
    }

    let file_name = output.file_name()?;
    let parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    let parent = parent.unwrap_or_else(|| Path::new("."));
    fs::canonicalize(parent)
        .ok()
        .map(|parent| parent.join(file_name))
}

fn collect_recite_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), CliError> {
    if path.is_file() {
        files.push(path.to_owned());
        return Ok(());
    }

    if !path.is_dir() {
        return Err(CliError::MissingPath(path.to_owned()));
    }

    for entry in fs::read_dir(path).map_err(|source| CliError::ReadDir {
        path: path.to_owned(),
        source,
    })? {
        let entry = entry.map_err(|source| CliError::ReadDir {
            path: path.to_owned(),
            source,
        })?;
        let child = entry.path();
        if child.is_dir() {
            collect_recite_files(&child, files)?;
        } else if child
            .extension()
            .is_some_and(|extension| extension == "recite")
        {
            files.push(child);
        }
    }

    Ok(())
}

struct LoadedSchema {
    schema: Option<ProjectSchema>,
    diagnostics: Vec<Diagnostic>,
}

fn load_optional_schema(
    schema_path: Option<&Path>,
    stderr: &mut dyn Write,
) -> Result<Option<ProjectSchema>, CliError> {
    let Some(schema_path) = schema_path else {
        return Ok(None);
    };

    let report = load_schema(schema_path)?;
    if !report.diagnostics.is_empty() {
        report_diagnostics(stderr, report.diagnostics.iter())?;
        return Err(CliError::Diagnostics);
    }

    Ok(report.schema)
}

fn load_schema(schema_path: &Path) -> Result<LoadedSchema, CliError> {
    let source = fs::read_to_string(schema_path).map_err(|source| CliError::Read {
        path: schema_path.to_owned(),
        source,
    })?;
    let report = load_schema_manifest_str(display_path(schema_path), &source);
    Ok(LoadedSchema {
        schema: report.schema,
        diagnostics: report.diagnostics,
    })
}

fn compile_options(
    output: &Path,
    schema: Option<&ProjectSchema>,
) -> Result<CompileOptions, CliError> {
    let output = display_path(output);
    let source_map = format!("{output}.map");
    Ok(CompileOptions::new(
        CompilerVersion::new(env!("CARGO_PKG_VERSION"))?,
        CompiledAssetId::new(output)?,
        SourceMapId::new(source_map)?,
        schema.map_or(
            SchemaFingerprint::NoSchema,
            ProjectSchema::canonical_fingerprint,
        ),
    ))
}

fn report_diagnostics<'a>(
    writer: &mut dyn Write,
    diagnostics: impl Iterator<Item = &'a Diagnostic>,
) -> Result<usize, CliError> {
    let mut count = 0;
    for diagnostic in diagnostics {
        count += 1;
        writeln!(
            writer,
            "{} {} {}:{}:{} {}",
            severity_name(diagnostic.severity),
            diagnostic.code.as_str(),
            diagnostic.span.file,
            diagnostic.span.start.line(),
            diagnostic.span.start.column(),
            diagnostic.message
        )?;
        for related in &diagnostic.related {
            writeln!(
                writer,
                "  related {}:{}:{} {}",
                related.span.file,
                related.span.start.line(),
                related.span.start.column(),
                related.message
            )?;
        }
        if let Some(help) = &diagnostic.help {
            writeln!(writer, "  help: {help}")?;
        }
    }
    Ok(count)
}

struct InputDiagnostics {
    parse_diagnostics: Vec<Diagnostic>,
    validation_diagnostics: Vec<Diagnostic>,
}

impl InputDiagnostics {
    fn into_all(self) -> Vec<Diagnostic> {
        self.parse_diagnostics
            .into_iter()
            .chain(self.validation_diagnostics)
            .collect()
    }
}

fn report_targeted_diagnostics(
    writer: &mut dyn Write,
    diagnostics: InputDiagnostics,
    is_target: impl Fn(&Diagnostic) -> bool,
) -> Result<(), CliError> {
    if !diagnostics.parse_diagnostics.is_empty() {
        report_diagnostics(writer, diagnostics.parse_diagnostics.iter())?;
        return Err(CliError::Diagnostics);
    }

    let targeted = diagnostics
        .validation_diagnostics
        .iter()
        .filter(|diagnostic| is_target(diagnostic))
        .collect::<Vec<_>>();
    if targeted.is_empty() {
        return Ok(());
    }

    report_diagnostics(writer, targeted.into_iter())?;
    Err(CliError::Diagnostics)
}

fn write_staged(output: &Path, contents: &[u8]) -> Result<(), CliError> {
    let temp_path = staged_output_path(output);
    if let Err(error) = fs::write(&temp_path, contents) {
        let _ = fs::remove_file(&temp_path);
        return Err(CliError::Write {
            path: temp_path,
            source: error,
        });
    }

    if let Err(error) = fs::rename(&temp_path, output) {
        let _ = fs::remove_file(&temp_path);
        return Err(CliError::Write {
            path: output.to_owned(),
            source: error,
        });
    }

    Ok(())
}

fn staged_output_path(output: &Path) -> PathBuf {
    let file_name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("recite-output");
    let temp_name = format!(".{file_name}.{}.tmp", std::process::id());

    match output.parent() {
        Some(parent) => parent.join(temp_name),
        None => PathBuf::from(temp_name),
    }
}

fn severity_name(severity: recite_core::DiagnosticSeverity) -> &'static str {
    match severity {
        recite_core::DiagnosticSeverity::Error => "error",
        recite_core::DiagnosticSeverity::Warning => "warning",
        recite_core::DiagnosticSeverity::Information => "info",
        recite_core::DiagnosticSeverity::Hint => "hint",
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn resolve_project_path(project_root: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_owned()
    } else {
        project_root.join(path)
    }
}

fn project_diagnostic(code: &str, message: impl Into<String>, span: SourceSpan) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::new(code).expect("project diagnostic codes are static and namespaced"),
        DiagnosticSeverity::Error,
        message,
        span,
    )
}

#[derive(Debug)]
enum CliError {
    Core(recite_core::CoreValueError),
    Compile(recite_compiler::CompileError),
    CompiledValue(recite_core::CompiledValueError),
    DecodeAsset {
        path: PathBuf,
        source: CompiledAssetDecodeError,
    },
    Diagnostics,
    FixtureChoiceIndexOutOfRange {
        index: usize,
        choice_count: usize,
        prompt_keys: Vec<String>,
    },
    FixtureChoiceNotInPrompt {
        choice: String,
        prompt_keys: Vec<String>,
    },
    FixtureToml {
        path: PathBuf,
        source: toml::de::Error,
    },
    Io(io::Error),
    MalformedCompiledAsset {
        reason: String,
    },
    MissingPath(PathBuf),
    MissingFixtureChoice {
        prompt_keys: Vec<String>,
    },
    NoInputs,
    OutputOverwritesInput {
        output: PathBuf,
        input: PathBuf,
    },
    Read {
        path: PathBuf,
        source: io::Error,
    },
    ReadDir {
        path: PathBuf,
        source: io::Error,
    },
    Runtime(recite_runtime::DialogueError),
    BlockingEffectNeedsAcknowledgement {
        effect: String,
    },
    TraceJson(serde_json::Error),
    UnknownPrompt {
        line: Option<String>,
        choices: Vec<String>,
    },
    Write {
        path: PathBuf,
        source: io::Error,
    },
}

impl std::fmt::Display for CliError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core(error) => write!(formatter, "{error}"),
            Self::Compile(error) => write!(formatter, "{error}"),
            Self::CompiledValue(error) => write!(formatter, "{error}"),
            Self::DecodeAsset { path, source } => {
                write!(
                    formatter,
                    "failed to decode compiled asset {}: {source}",
                    display_path(path)
                )
            }
            Self::Diagnostics => formatter.write_str("diagnostics reported"),
            Self::FixtureChoiceIndexOutOfRange {
                index,
                choice_count,
                prompt_keys,
            } => write!(
                formatter,
                "fixture choice index {index} is out of range for prompt {} with {choice_count} choices; indexes are 1-based",
                prompt_keys.join("|")
            ),
            Self::FixtureChoiceNotInPrompt {
                choice,
                prompt_keys,
            } => write!(
                formatter,
                "fixture choice `{choice}` is not in prompt {}",
                prompt_keys.join("|")
            ),
            Self::FixtureToml { path, source } => {
                write!(
                    formatter,
                    "failed to parse fixture {}: {source}",
                    display_path(path)
                )
            }
            Self::Io(error) => write!(formatter, "{error}"),
            Self::MalformedCompiledAsset { reason } => {
                write!(formatter, "malformed compiled asset: {reason}")
            }
            Self::MissingPath(path) => write!(
                formatter,
                "input path does not exist: {}",
                display_path(path)
            ),
            Self::MissingFixtureChoice { prompt_keys } => write!(
                formatter,
                "fixture is missing a [choices] entry for prompt {}; supported keys for this prompt are listed in trace prompt.identity.fixture_keys",
                prompt_keys.join("|")
            ),
            Self::NoInputs => formatter.write_str("no .recite inputs found"),
            Self::OutputOverwritesInput { output, input } => write!(
                formatter,
                "refusing to overwrite input {} with output {}",
                display_path(input),
                display_path(output)
            ),
            Self::Read { path, source } => {
                write!(formatter, "failed to read {}: {source}", display_path(path))
            }
            Self::ReadDir { path, source } => {
                write!(
                    formatter,
                    "failed to read directory {}: {source}",
                    display_path(path)
                )
            }
            Self::Runtime(error) => write!(formatter, "{error}"),
            Self::BlockingEffectNeedsAcknowledgement { effect } => write!(
                formatter,
                "blocking effect `{effect}` requires [effects].auto_ack_blocking = true in the fixture"
            ),
            Self::TraceJson(error) => write!(formatter, "failed to encode trace JSON: {error}"),
            Self::UnknownPrompt { line, choices } => write!(
                formatter,
                "runtime emitted an unknown prompt line={} choices=[{}]",
                line.as_deref().unwrap_or("<none>"),
                choices.join(", ")
            ),
            Self::Write { path, source } => {
                write!(
                    formatter,
                    "failed to write {}: {source}",
                    display_path(path)
                )
            }
        }
    }
}

impl std::error::Error for CliError {}

impl From<io::Error> for CliError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<recite_core::CoreValueError> for CliError {
    fn from(error: recite_core::CoreValueError) -> Self {
        Self::Core(error)
    }
}

impl From<recite_core::CompiledValueError> for CliError {
    fn from(error: recite_core::CompiledValueError) -> Self {
        Self::CompiledValue(error)
    }
}

impl From<recite_compiler::CompileError> for CliError {
    fn from(error: recite_compiler::CompileError) -> Self {
        Self::Compile(error)
    }
}

impl From<recite_runtime::DialogueError> for CliError {
    fn from(error: recite_runtime::DialogueError) -> Self {
        Self::Runtime(error)
    }
}
