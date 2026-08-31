use recite_runtime::{
    DialogueChoice, DialogueEffectArgument, DialogueEffectRequest, DialogueLine, PreviewEvent,
    PreviewPrompt, PreviewSession, PreviewSnapshot, PreviewTrace,
};

use crate::{BenchmarkResult, error};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewRetentionReport {
    pub fixture: &'static str,
    pub snapshot: PreviewSnapshotShape,
    pub trace: PreviewTraceShape,
    pub transcript_events: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewSnapshotShape {
    pub encoded_bytes: usize,
    pub selected_choice_count: usize,
    pub deferred_effect_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewTraceShape {
    pub event_count: usize,
    pub condition_request_count: usize,
    pub condition_result_count: usize,
    pub line_count: usize,
    pub prompt_count: usize,
    pub choice_accepted_count: usize,
    pub choice_selected_count: usize,
    pub effect_count: usize,
    pub deferred_effect_count: usize,
    pub end_count: usize,
    /// A lower bound for retained trace storage. It includes event slots and
    /// selected textual payloads, but excludes Vec capacity and allocator
    /// overhead; it is a shape probe, not an exact heap measurement.
    pub retained_bytes_lower_bound: usize,
    pub localized_lookup_count: usize,
    pub plural_line_count: usize,
}

pub(crate) fn build_report(
    fixture: &'static str,
    preview: &PreviewSession<'_>,
) -> BenchmarkResult<PreviewRetentionReport> {
    let snapshot = preview.snapshot().map_err(preview_error)?;
    let snapshot_encoded_bytes = snapshot.encode().map_err(preview_error)?.len();
    Ok(PreviewRetentionReport {
        fixture,
        snapshot: snapshot_shape(&snapshot, snapshot_encoded_bytes),
        trace: trace_shape(preview.trace()),
        transcript_events: preview.transcript().events().len(),
    })
}

fn preview_error(preview: recite_runtime::PreviewError) -> crate::BenchmarkError {
    error(format!("preview operation failed: {preview}"))
}

fn snapshot_shape(snapshot: &PreviewSnapshot, encoded_bytes: usize) -> PreviewSnapshotShape {
    PreviewSnapshotShape {
        encoded_bytes,
        selected_choice_count: snapshot.state().selected_choice_history().len(),
        deferred_effect_count: snapshot.state().deferred_effects().len(),
    }
}

fn trace_shape(trace: &PreviewTrace) -> PreviewTraceShape {
    let mut shape = PreviewTraceShape {
        event_count: trace.events().len(),
        condition_request_count: 0,
        condition_result_count: 0,
        line_count: 0,
        prompt_count: 0,
        choice_accepted_count: 0,
        choice_selected_count: 0,
        effect_count: 0,
        deferred_effect_count: 0,
        end_count: 0,
        retained_bytes_lower_bound: std::mem::size_of_val(trace.events()),
        localized_lookup_count: trace.localized_lookups().count(),
        plural_line_count: trace.plural_lines().count(),
    };
    for event in trace.events() {
        match event {
            PreviewEvent::ConditionRequested(request) => {
                shape.condition_request_count += 1;
                shape.retained_bytes_lower_bound += request.query().function().len();
            }
            PreviewEvent::ConditionResult { request, .. } => {
                shape.condition_result_count += 1;
                shape.retained_bytes_lower_bound += request.query().function().len();
            }
            PreviewEvent::Line(line) => {
                shape.line_count += 1;
                shape.retained_bytes_lower_bound += line_payload_bytes(line);
            }
            PreviewEvent::Prompt(prompt) => {
                shape.prompt_count += 1;
                shape.retained_bytes_lower_bound += prompt_payload_bytes(prompt);
            }
            PreviewEvent::ChoiceAccepted { .. } => shape.choice_accepted_count += 1,
            PreviewEvent::ChoiceSelected { .. } => shape.choice_selected_count += 1,
            PreviewEvent::EffectRequested(effect) => {
                shape.effect_count += 1;
                shape.retained_bytes_lower_bound += effect_payload_bytes(effect);
            }
            PreviewEvent::DeferredEffectScheduled(effect) => {
                shape.deferred_effect_count += 1;
                shape.retained_bytes_lower_bound += effect_payload_bytes(effect);
            }
            PreviewEvent::End { deferred_effects } => {
                shape.end_count += 1;
                shape.retained_bytes_lower_bound += deferred_effects
                    .iter()
                    .map(effect_payload_bytes)
                    .sum::<usize>();
            }
            PreviewEvent::EffectAcknowledged { .. }
            | PreviewEvent::Restarted { .. }
            | PreviewEvent::Restored
            | PreviewEvent::RestartRequired { .. }
            | PreviewEvent::Error(_) => {}
            _ => {}
        }
    }
    shape
}

fn line_payload_bytes(line: &DialogueLine) -> usize {
    line.id.as_str().len()
        + line.source_text.len()
        + line.text.len()
        + line
            .speaker
            .as_ref()
            .map_or(0, |speaker| speaker.as_str().len())
        + line
            .metadata
            .iter()
            .map(|entry| entry.key.len())
            .sum::<usize>()
}

fn prompt_payload_bytes(prompt: &PreviewPrompt) -> usize {
    let identity = prompt.identity();
    identity.block().as_str().len()
        + identity.line().map_or(0, |line| line.as_str().len())
        + identity
            .choices()
            .iter()
            .map(|choice| choice.as_str().len())
            .sum::<usize>()
        + prompt.line().map_or(0, line_payload_bytes)
        + prompt
            .choices()
            .iter()
            .map(choice_payload_bytes)
            .sum::<usize>()
}

fn choice_payload_bytes(choice: &DialogueChoice) -> usize {
    choice.id.as_str().len()
        + choice.source_text.len()
        + choice.text.len()
        + choice
            .metadata
            .iter()
            .map(|entry| entry.key.len())
            .sum::<usize>()
}

fn effect_payload_bytes(effect: &DialogueEffectRequest) -> usize {
    effect.id.as_str().len()
        + effect.function.len()
        + effect.args.iter().map(effect_argument_bytes).sum::<usize>()
}

fn effect_argument_bytes(argument: &DialogueEffectArgument) -> usize {
    match argument {
        DialogueEffectArgument::Identifier(value) | DialogueEffectArgument::String(value) => {
            value.len()
        }
        DialogueEffectArgument::Boolean(value) => value.to_string().len(),
        DialogueEffectArgument::Integer(value) => value.to_string().len(),
        DialogueEffectArgument::Float(value) => value.to_string().len(),
    }
}
