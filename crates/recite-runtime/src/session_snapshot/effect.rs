//! Self-contained effect requests in snapshots.

use recite_core::{EffectId, SourcePosition, SourceSpan};
use serde::{Serialize, Serializer};

use crate::{DialogueEffectArgument, DialogueEffectMode, DialogueEffectRequest};

use super::{
    DialogueEffectArgumentSnapshot, DialogueEffectModeSnapshot, DialogueEffectRequestSnapshot,
    DialogueSessionSnapshotConversionError, DialogueSourcePositionSnapshot,
    DialogueSourceSpanSnapshot,
};

#[must_use]
pub fn effect_request_snapshot(request: &DialogueEffectRequest) -> DialogueEffectRequestSnapshot {
    DialogueEffectRequestSnapshot {
        id: request.id.as_str().to_owned(),
        mode: effect_mode_snapshot(request.mode),
        function: request.function.clone(),
        args: request.args.iter().map(argument_snapshot).collect(),
        source_span: span_snapshot(&request.source_span),
    }
}

pub fn effect_request_from_snapshot(
    snapshot: &DialogueEffectRequestSnapshot,
) -> Result<DialogueEffectRequest, DialogueSessionSnapshotConversionError> {
    let invalid = |reason: String| DialogueSessionSnapshotConversionError::InvalidEffectRequest {
        id: snapshot.id.clone(),
        reason,
    };
    let id = EffectId::new(snapshot.id.clone()).map_err(|error| invalid(error.to_string()))?;
    let span = &snapshot.source_span;
    let start = SourcePosition::new(span.start_line, span.start_column)
        .map_err(|error| invalid(error.to_string()))?;
    let end = span
        .end
        .map(|end| SourcePosition::new(end.line, end.column))
        .transpose()
        .map_err(|error| invalid(error.to_string()))?;
    Ok(DialogueEffectRequest {
        id,
        mode: match snapshot.mode {
            DialogueEffectModeSnapshot::Deferred => DialogueEffectMode::Deferred,
            DialogueEffectModeSnapshot::Immediate => DialogueEffectMode::Immediate,
            DialogueEffectModeSnapshot::Blocking => DialogueEffectMode::Blocking,
        },
        function: snapshot.function.clone(),
        args: snapshot
            .args
            .iter()
            .map(|argument| match argument {
                DialogueEffectArgumentSnapshot::Identifier(value) => {
                    DialogueEffectArgument::Identifier(value.clone())
                }
                DialogueEffectArgumentSnapshot::String(value) => {
                    DialogueEffectArgument::String(value.clone())
                }
                DialogueEffectArgumentSnapshot::Integer(value) => {
                    DialogueEffectArgument::Integer(*value)
                }
                DialogueEffectArgumentSnapshot::Float(value) => {
                    DialogueEffectArgument::Float(*value)
                }
                DialogueEffectArgumentSnapshot::Boolean(value) => {
                    DialogueEffectArgument::Boolean(*value)
                }
            })
            .collect(),
        source_span: SourceSpan::new(span.file.clone(), start, end),
    })
}

fn effect_mode_snapshot(mode: DialogueEffectMode) -> DialogueEffectModeSnapshot {
    match mode {
        DialogueEffectMode::Deferred => DialogueEffectModeSnapshot::Deferred,
        DialogueEffectMode::Immediate => DialogueEffectModeSnapshot::Immediate,
        DialogueEffectMode::Blocking => DialogueEffectModeSnapshot::Blocking,
    }
}

fn argument_snapshot(argument: &DialogueEffectArgument) -> DialogueEffectArgumentSnapshot {
    match argument {
        DialogueEffectArgument::Identifier(value) => {
            DialogueEffectArgumentSnapshot::Identifier(value.clone())
        }
        DialogueEffectArgument::String(value) => {
            DialogueEffectArgumentSnapshot::String(value.clone())
        }
        DialogueEffectArgument::Integer(value) => DialogueEffectArgumentSnapshot::Integer(*value),
        DialogueEffectArgument::Float(value) => DialogueEffectArgumentSnapshot::Float(*value),
        DialogueEffectArgument::Boolean(value) => DialogueEffectArgumentSnapshot::Boolean(*value),
    }
}

fn span_snapshot(span: &SourceSpan) -> DialogueSourceSpanSnapshot {
    DialogueSourceSpanSnapshot {
        file: span.file.clone(),
        start_line: span.start.line(),
        start_column: span.start.column(),
        end: span.end.map(|end| DialogueSourcePositionSnapshot {
            line: end.line(),
            column: end.column(),
        }),
    }
}

/// Mirrors [`DialogueEffectRequestSnapshot`] without copying the request.
#[derive(Serialize)]
pub(crate) struct EffectRequestRef<'a> {
    id: &'a str,
    mode: DialogueEffectModeSnapshot,
    function: &'a str,
    #[serde(serialize_with = "arguments")]
    args: &'a [DialogueEffectArgument],
    source_span: SourceSpanRef<'a>,
}

#[derive(Serialize)]
struct SourceSpanRef<'a> {
    file: &'a str,
    start_line: u32,
    start_column: u32,
    end: Option<DialogueSourcePositionSnapshot>,
}

impl<'a> EffectRequestRef<'a> {
    pub(crate) fn new(request: &'a DialogueEffectRequest) -> Self {
        let span = &request.source_span;
        Self {
            id: request.id.as_str(),
            mode: effect_mode_snapshot(request.mode),
            function: &request.function,
            args: &request.args,
            source_span: SourceSpanRef {
                file: &span.file,
                start_line: span.start.line(),
                start_column: span.start.column(),
                end: span.end.map(|end| DialogueSourcePositionSnapshot {
                    line: end.line(),
                    column: end.column(),
                }),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
enum ArgumentRef<'a> {
    Identifier(&'a str),
    String(&'a str),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

fn arguments<S: Serializer>(
    arguments: &&[DialogueEffectArgument],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(arguments.iter().map(|argument| match argument {
        DialogueEffectArgument::Identifier(value) => ArgumentRef::Identifier(value),
        DialogueEffectArgument::String(value) => ArgumentRef::String(value),
        DialogueEffectArgument::Integer(value) => ArgumentRef::Integer(*value),
        DialogueEffectArgument::Float(value) => ArgumentRef::Float(*value),
        DialogueEffectArgument::Boolean(value) => ArgumentRef::Boolean(*value),
    }))
}
