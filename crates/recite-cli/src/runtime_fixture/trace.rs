use recite_core::{MetadataEntry, ScalarValue, SourceSpan, Value};
use recite_runtime::{
    ConditionArgument, DialogueChoice, DialogueEffectArgument, DialogueEffectMode,
    DialogueEffectRequest, DialogueLine,
};
use serde::Serialize;

use crate::runtime_format::{
    RuntimeDisplayArgument, format_condition_query, format_effect_arguments as format_effect_args,
};

pub(super) fn trace_line(line: &DialogueLine) -> TraceLine {
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

pub(super) fn trace_choice(choice: &DialogueChoice) -> TraceChoice {
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

pub(super) fn trace_effect(effect: &DialogueEffectRequest) -> TraceEffect {
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

pub(super) fn trace_condition_argument(argument: ConditionArgument<'_>) -> TraceScalar {
    match argument {
        ConditionArgument::Identifier(value) => TraceScalar::Identifier(value.to_owned()),
        ConditionArgument::String(value) => TraceScalar::String(value.to_owned()),
        ConditionArgument::Integer(value) => TraceScalar::Integer(value),
        ConditionArgument::Float(value) => TraceScalar::Float(value),
        ConditionArgument::Boolean(value) => TraceScalar::Boolean(value),
    }
}

pub(super) fn trace_effect_argument(argument: &DialogueEffectArgument) -> TraceScalar {
    match argument {
        DialogueEffectArgument::Identifier(value) => TraceScalar::Identifier(value.clone()),
        DialogueEffectArgument::String(value) => TraceScalar::String(value.clone()),
        DialogueEffectArgument::Integer(value) => TraceScalar::Integer(*value),
        DialogueEffectArgument::Float(value) => TraceScalar::Float(*value),
        DialogueEffectArgument::Boolean(value) => TraceScalar::Boolean(*value),
    }
}

pub(super) fn condition_query_text(function: &str, arguments: &[TraceScalar]) -> String {
    format_condition_query(
        function,
        arguments.iter().map(trace_scalar_display_argument),
    )
}

fn trace_scalar_display_argument(argument: &TraceScalar) -> RuntimeDisplayArgument<'_> {
    match argument {
        TraceScalar::Identifier(value) => RuntimeDisplayArgument::Identifier(value),
        TraceScalar::String(value) => RuntimeDisplayArgument::String(value),
        TraceScalar::Integer(value) => RuntimeDisplayArgument::Integer(*value),
        TraceScalar::Float(value) => RuntimeDisplayArgument::Float(*value),
        TraceScalar::Boolean(value) => RuntimeDisplayArgument::Boolean(*value),
    }
}

pub(super) fn format_effect_arguments(arguments: &[DialogueEffectArgument]) -> String {
    format_effect_args(arguments)
}

fn effect_mode_name(mode: DialogueEffectMode) -> &'static str {
    match mode {
        DialogueEffectMode::Deferred => "deferred",
        DialogueEffectMode::Immediate => "immediate",
        DialogueEffectMode::Blocking => "blocking",
    }
}

#[derive(Serialize)]
pub(crate) struct TraceDocument {
    asset_id: String,
    block: String,
    events: Vec<TraceEvent>,
    final_deferred_effects: Vec<TraceEffect>,
}

impl TraceDocument {
    pub(super) fn new(
        asset_id: String,
        block: String,
        events: Vec<TraceEvent>,
        final_deferred_effects: Vec<TraceEffect>,
    ) -> Self {
        Self {
            asset_id,
            block,
            events,
            final_deferred_effects,
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum TraceEvent {
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
pub(super) struct TraceCondition {
    pub(super) query: String,
    pub(super) function: String,
    pub(super) arguments: Vec<TraceScalar>,
    pub(super) result: TraceConditionValue,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub(super) enum TraceConditionValue {
    Bool(bool),
    EnumVariant { r#enum: String },
}

impl std::fmt::Display for TraceConditionValue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(value) => write!(formatter, "{value}"),
            Self::EnumVariant { r#enum } => write!(formatter, "enum {enum}"),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct TracePrompt {
    pub(super) identity: TracePromptIdentity,
    pub(super) line: Option<TraceLine>,
    pub(super) choices: Vec<TraceChoice>,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct TracePromptIdentity {
    pub(super) block: String,
    pub(super) line: Option<String>,
    pub(super) fixture_keys: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct TraceLine {
    id: String,
    source_text: String,
    text: String,
    speaker: Option<String>,
    metadata: Vec<TraceMetadata>,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct TraceChoice {
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
pub(super) enum TraceScalar {
    Identifier(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct TraceEffect {
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
