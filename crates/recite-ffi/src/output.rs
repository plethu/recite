use recite_core::{ScalarValue, Value};
use recite_runtime::{
    ChoiceAvailability, ChoiceAvailabilityReason, ChoiceAvailabilityReasonTree,
    ChoiceAvailabilityReasonValue, ChoiceEchoMode, DialogueEffectArgument, DialogueEffectMode,
    DialogueEffectRequest, DialogueEvent, DialogueLine,
};
use serde::Serialize;
use std::fmt;
use std::io::Write;

pub(crate) const BATCH_FORMAT_VERSION: u16 = 0;

#[derive(Serialize)]
pub(crate) struct FfiOutputBatch {
    pub batch_format_version: u16,
    pub events: Vec<FfiEvent>,
}

/// Serialization failed while building a host-facing output batch.
///
/// This remains private to the FFI implementation. The C ABI flattens it to
/// the stable dialogue-fault status only after the encoder has returned.
#[derive(Debug)]
pub(crate) struct FfiOutputEncodeError {
    source: rmp_serde::encode::Error,
}

impl fmt::Display for FfiOutputEncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "failed to encode FFI output batch: {}",
            self.source
        )
    }
}

impl std::error::Error for FfiOutputEncodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FfiEvent {
    Line(FfiLine),
    Prompt {
        line: Option<FfiLine>,
        choices: Vec<FfiChoice>,
    },
    Effect(FfiEffect),
    End {
        deferred_effects: Vec<FfiEffect>,
    },
}

#[derive(Serialize)]
pub(crate) struct FfiLine {
    pub id: String,
    pub source_text: String,
    pub text: String,
    pub speaker: Option<String>,
    pub metadata: Vec<FfiMetadata>,
}

#[derive(Serialize)]
pub(crate) struct FfiMetadata {
    pub key: String,
    pub value: FfiMetaValue,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FfiMetaValue {
    String { value: String },
    Integer { value: i64 },
    Float { value: f64 },
    Boolean { value: bool },
    Array { values: Vec<FfiScalar> },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FfiScalar {
    String { value: String },
    Integer { value: i64 },
    Float { value: f64 },
    Boolean { value: bool },
}

#[derive(Serialize)]
pub(crate) struct FfiChoice {
    pub id: String,
    pub source_text: String,
    pub text: String,
    pub metadata: Vec<FfiMetadata>,
    pub echo: FfiEcho,
    pub availability: FfiAvailability,
}

#[derive(Serialize)]
pub(crate) struct FfiEcho {
    pub kind: &'static str,
    pub explicit_line_id: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct FfiAvailability {
    pub is_available: bool,
    pub primary_reason: Option<FfiAvailabilityReason>,
    pub reason_tree: Option<FfiReasonTree>,
}

#[derive(Serialize)]
pub(crate) struct FfiAvailabilityReason {
    pub id: String,
    pub source_text: String,
    pub text: String,
    pub args: Vec<FfiReasonArg>,
}

#[derive(Serialize)]
pub(crate) struct FfiReasonArg {
    pub name: String,
    pub value: FfiReasonValue,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FfiReasonValue {
    Identifier { value: String },
    String { value: String },
    Integer { value: i64 },
    Float { value: f64 },
    Boolean { value: bool },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FfiReasonTree {
    All { children: Vec<FfiReasonTree> },
    Any { children: Vec<FfiReasonTree> },
    Reason(FfiAvailabilityReason),
    RequirementSourceText { text: String },
}

#[derive(Serialize)]
pub(crate) struct FfiEffect {
    pub id: String,
    pub mode: &'static str,
    pub function: String,
    pub args: Vec<FfiEffectArg>,
    pub source_file: String,
    pub source_line: u32,
    pub source_col: u32,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum FfiEffectArg {
    Identifier { value: String },
    String { value: String },
    Integer { value: i64 },
    Float { value: f64 },
    Boolean { value: bool },
}

pub(crate) fn encode_batch(events: Vec<DialogueEvent>) -> Result<Vec<u8>, FfiOutputEncodeError> {
    let mut bytes = Vec::new();
    encode_batch_to_writer(events, &mut bytes)?;
    Ok(bytes)
}

fn encode_batch_to_writer<W: Write + ?Sized>(
    events: Vec<DialogueEvent>,
    writer: &mut W,
) -> Result<(), FfiOutputEncodeError> {
    let ffi_events: Vec<FfiEvent> = events.into_iter().map(ffi_event).collect();
    let batch = FfiOutputBatch {
        batch_format_version: BATCH_FORMAT_VERSION,
        events: ffi_events,
    };
    rmp_serde::encode::write_named(writer, &batch).map_err(|source| FfiOutputEncodeError { source })
}

#[cfg(test)]
mod tests;

fn ffi_event(event: DialogueEvent) -> FfiEvent {
    match event {
        DialogueEvent::Line(line) => FfiEvent::Line(ffi_line(line)),
        DialogueEvent::Prompt { line, choices } => FfiEvent::Prompt {
            line: line.map(ffi_line),
            choices: choices.into_iter().map(ffi_choice).collect(),
        },
        DialogueEvent::Effect(effect) => FfiEvent::Effect(ffi_effect(effect)),
        DialogueEvent::End { deferred_effects } => FfiEvent::End {
            deferred_effects: deferred_effects.into_iter().map(ffi_effect).collect(),
        },
    }
}

fn ffi_line(line: DialogueLine) -> FfiLine {
    FfiLine {
        id: line.id.as_str().to_owned(),
        source_text: line.source_text,
        text: line.text,
        speaker: line.speaker.map(|s| s.as_str().to_owned()),
        metadata: line.metadata.into_iter().map(ffi_metadata).collect(),
    }
}

fn ffi_choice(choice: recite_runtime::DialogueChoice) -> FfiChoice {
    FfiChoice {
        id: choice.id.as_str().to_owned(),
        source_text: choice.source_text,
        text: choice.text,
        metadata: choice.metadata.into_iter().map(ffi_metadata).collect(),
        echo: ffi_echo(choice.echo),
        availability: ffi_availability(choice.availability),
    }
}

fn ffi_echo(echo: ChoiceEchoMode) -> FfiEcho {
    match echo {
        ChoiceEchoMode::None => FfiEcho {
            kind: "none",
            explicit_line_id: None,
        },
        ChoiceEchoMode::SelectedText => FfiEcho {
            kind: "selected_text",
            explicit_line_id: None,
        },
        ChoiceEchoMode::ExplicitLine(id) => FfiEcho {
            kind: "explicit_line",
            explicit_line_id: Some(id.as_str().to_owned()),
        },
    }
}

fn ffi_availability(av: ChoiceAvailability) -> FfiAvailability {
    FfiAvailability {
        is_available: av.is_available,
        primary_reason: av.primary_reason.map(ffi_availability_reason),
        reason_tree: av.reason_tree.map(ffi_reason_tree),
    }
}

fn ffi_availability_reason(reason: ChoiceAvailabilityReason) -> FfiAvailabilityReason {
    FfiAvailabilityReason {
        id: reason.id.as_str().to_owned(),
        source_text: reason.source_text,
        text: reason.text,
        args: reason
            .args
            .into_iter()
            .map(|arg| FfiReasonArg {
                name: arg.name,
                value: ffi_reason_value(arg.value),
            })
            .collect(),
    }
}

fn ffi_reason_tree(tree: ChoiceAvailabilityReasonTree) -> FfiReasonTree {
    match tree {
        ChoiceAvailabilityReasonTree::All(children) => FfiReasonTree::All {
            children: children.into_iter().map(ffi_reason_tree).collect(),
        },
        ChoiceAvailabilityReasonTree::Any(children) => FfiReasonTree::Any {
            children: children.into_iter().map(ffi_reason_tree).collect(),
        },
        ChoiceAvailabilityReasonTree::Reason(reason) => {
            FfiReasonTree::Reason(ffi_availability_reason(reason))
        }
        ChoiceAvailabilityReasonTree::RequirementSourceText(text) => {
            FfiReasonTree::RequirementSourceText { text }
        }
    }
}

fn ffi_reason_value(value: ChoiceAvailabilityReasonValue) -> FfiReasonValue {
    match value {
        ChoiceAvailabilityReasonValue::Identifier(v) => FfiReasonValue::Identifier { value: v },
        ChoiceAvailabilityReasonValue::String(v) => FfiReasonValue::String { value: v },
        ChoiceAvailabilityReasonValue::Integer(v) => FfiReasonValue::Integer { value: v },
        ChoiceAvailabilityReasonValue::Float(v) => FfiReasonValue::Float { value: v },
        ChoiceAvailabilityReasonValue::Boolean(v) => FfiReasonValue::Boolean { value: v },
    }
}

fn ffi_effect(effect: DialogueEffectRequest) -> FfiEffect {
    let mode = match effect.mode {
        DialogueEffectMode::Deferred => "deferred",
        DialogueEffectMode::Immediate => "immediate",
        DialogueEffectMode::Blocking => "blocking",
    };
    FfiEffect {
        id: effect.id.as_str().to_owned(),
        mode,
        function: effect.function,
        args: effect.args.into_iter().map(ffi_effect_arg).collect(),
        source_file: effect.source_span.file,
        source_line: effect.source_span.start.line(),
        source_col: effect.source_span.start.column(),
    }
}

fn ffi_effect_arg(arg: DialogueEffectArgument) -> FfiEffectArg {
    match arg {
        DialogueEffectArgument::Identifier(v) => FfiEffectArg::Identifier { value: v },
        DialogueEffectArgument::String(v) => FfiEffectArg::String { value: v },
        DialogueEffectArgument::Integer(v) => FfiEffectArg::Integer { value: v },
        DialogueEffectArgument::Float(v) => FfiEffectArg::Float { value: v },
        DialogueEffectArgument::Boolean(v) => FfiEffectArg::Boolean { value: v },
    }
}

fn ffi_metadata(entry: recite_core::MetadataEntry) -> FfiMetadata {
    FfiMetadata {
        key: entry.key,
        value: ffi_meta_value(entry.value),
    }
}

fn ffi_meta_value(value: Value) -> FfiMetaValue {
    match value {
        Value::Scalar(scalar) => ffi_scalar_as_meta(scalar),
        Value::Array(items) => FfiMetaValue::Array {
            values: items.into_iter().map(ffi_scalar).collect(),
        },
    }
}

fn ffi_scalar_as_meta(scalar: ScalarValue) -> FfiMetaValue {
    match scalar {
        ScalarValue::String(v) => FfiMetaValue::String { value: v },
        ScalarValue::Integer(v) => FfiMetaValue::Integer { value: v },
        ScalarValue::Float(v) => FfiMetaValue::Float { value: v },
        ScalarValue::Boolean(v) => FfiMetaValue::Boolean { value: v },
    }
}

fn ffi_scalar(scalar: ScalarValue) -> FfiScalar {
    match scalar {
        ScalarValue::String(v) => FfiScalar::String { value: v },
        ScalarValue::Integer(v) => FfiScalar::Integer { value: v },
        ScalarValue::Float(v) => FfiScalar::Float { value: v },
        ScalarValue::Boolean(v) => FfiScalar::Boolean { value: v },
    }
}

/// Returns true for events that do not stop the drain loop.
///
/// Lines and immediate effects can be followed by more output. Prompts,
/// blocking effects, end events, and errors stop the batch at the boundary.
pub(crate) fn should_continue(event: &DialogueEvent) -> bool {
    matches!(
        event,
        DialogueEvent::Line(_)
            | DialogueEvent::Effect(DialogueEffectRequest {
                mode: DialogueEffectMode::Immediate,
                ..
            })
    )
}
