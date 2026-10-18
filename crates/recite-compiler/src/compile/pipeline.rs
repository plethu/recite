//! Both raw and immutable build inputs pass through the same validation pipeline.
use recite_core::{Diagnostic, compiled::CompiledDialogue, schema::ProjectSchema};
use recite_parser::lower_source_file;

use super::{
    CompileError,
    api::{CompileInput, CompileOptions},
    builder::build_dialogue,
    lowered::LoweredInput,
};
use crate::{
    authoring::BuildInput,
    validation::{
        ProjectCompleteness, ValidationInput, project::sort_diagnostics_by_source, validate_inputs,
    },
};

pub(super) enum Input<'a> {
    Raw(CompileInput),
    Captured(&'a BuildInput),
}

pub(super) struct PipelineReport<T> {
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) output: Option<T>,
}

pub(super) fn compile_inputs_with_output<'a, T>(
    inputs: impl IntoIterator<Item = Input<'a>>,
    options: CompileOptions,
    schema: Option<&ProjectSchema>,
    emit: impl FnOnce(CompiledDialogue, &[LoweredInput]) -> Result<T, CompileError>,
) -> Result<PipelineReport<T>, CompileError> {
    let prepared = prepare_inputs(inputs, schema);
    let Some(lowered_inputs) = prepared.output else {
        return Ok(PipelineReport {
            diagnostics: prepared.diagnostics,
            output: None,
        });
    };
    let dialogue = build_dialogue(&lowered_inputs, options, schema)?;
    Ok(PipelineReport {
        diagnostics: Vec::new(),
        output: Some(emit(dialogue, &lowered_inputs)?),
    })
}

pub(super) fn prepare_inputs<'a>(
    inputs: impl IntoIterator<Item = Input<'a>>,
    schema: Option<&ProjectSchema>,
) -> PipelineReport<Vec<LoweredInput>> {
    let mut lowered_inputs = Vec::new();
    let mut diagnostics = Vec::new();

    for (input_index, input) in inputs.into_iter().enumerate() {
        let (lowered, source, fingerprint) = match input {
            Input::Raw(input) => (
                lower_source_file(&input.path, &input.source),
                input.source.into(),
                None,
            ),
            Input::Captured(input) => {
                let Some(source) = input.content() else {
                    continue;
                };
                (
                    lower_source_file(input.key().as_str(), source),
                    std::sync::Arc::from(source),
                    Some(input.fingerprint().clone()),
                )
            }
        };
        diagnostics.extend(lowered.diagnostics);
        lowered_inputs.push(LoweredInput {
            input_index,
            source,
            fingerprint,
            source_file: lowered.source_file,
        });
    }

    sort_diagnostics_by_source(&mut diagnostics);
    if !diagnostics.is_empty() {
        return PipelineReport {
            diagnostics,
            output: None,
        };
    }

    let validation = validate_inputs(
        lowered_inputs
            .iter()
            .map(|input| ValidationInput::all_complete(&input.source_file)),
        schema,
        ProjectCompleteness::Complete,
    );
    if !validation.is_ok() {
        return PipelineReport {
            diagnostics: validation.diagnostics,
            output: None,
        };
    }

    lowered_inputs.sort_by(|left, right| {
        left.source_file
            .path
            .cmp(&right.source_file.path)
            .then(left.input_index.cmp(&right.input_index))
    });

    PipelineReport {
        diagnostics: Vec::new(),
        output: Some(lowered_inputs),
    }
}
