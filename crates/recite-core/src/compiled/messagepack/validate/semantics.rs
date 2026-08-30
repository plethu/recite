use crate::{ScalarValue, SourceSpan, Value};

use super::super::tags::ensure_identifier_like;
use super::tables::{ensure_index, ensure_range};
use super::{CompiledAssetDecodeError, malformed};
use crate::compiled::{
    CompiledArgument, CompiledAvailabilityReasonArgValue, CompiledChoiceEcho,
    CompiledConditionExpression, CompiledDialogue, CompiledDivertTarget, CompiledStatement,
    CompiledStatementKind, MatchArmIndex,
};

pub(super) fn validate_statement(
    dialogue: &CompiledDialogue,
    statement: &CompiledStatement,
) -> Result<(), CompiledAssetDecodeError> {
    match &statement.kind {
        CompiledStatementKind::Line(index) => {
            ensure_index("line statement", dialogue.lines.len(), index.as_u32())?;
        }
        CompiledStatementKind::Prompt { line, choices } => {
            if let Some(index) = line {
                ensure_index("prompt line", dialogue.lines.len(), index.as_u32())?;
            }
            if choices.len == 0 {
                return Err(malformed("prompt choices must not be empty".to_owned()));
            }
            ensure_range(
                "prompt choices",
                dialogue.choices.len(),
                *choices,
                |index| index.as_u32(),
            )?;
        }
        CompiledStatementKind::Divert(target) => validate_divert(dialogue, target)?,
        CompiledStatementKind::If {
            condition,
            then_statements,
            else_statements,
        } => {
            validate_condition(condition)?;
            ensure_range(
                "if then statements",
                dialogue.statements.len(),
                *then_statements,
                |index| index.as_u32(),
            )?;
            ensure_range(
                "if else statements",
                dialogue.statements.len(),
                *else_statements,
                |index| index.as_u32(),
            )?;
        }
        CompiledStatementKind::Match { scrutinee, arms } => {
            validate_identifier("condition function", &scrutinee.function)?;
            for argument in &scrutinee.args {
                validate_argument(argument)?;
            }
            ensure_range(
                "match arms",
                dialogue.match_arms.len(),
                *arms,
                |index: MatchArmIndex| index.as_u32(),
            )?;
        }
        CompiledStatementKind::Effect(index) => {
            ensure_index("effect statement", dialogue.effects.len(), index.as_u32())?;
        }
        CompiledStatementKind::End => {}
    }
    ensure_index(
        "statement source map",
        dialogue.source_maps.len(),
        statement.source_map.as_u32(),
    )?;
    Ok(())
}

pub(super) fn validate_divert(
    dialogue: &CompiledDialogue,
    target: &CompiledDivertTarget,
) -> Result<(), CompiledAssetDecodeError> {
    if let CompiledDivertTarget::Block(index) = target {
        ensure_index("block divert", dialogue.blocks.len(), index.as_u32())?;
    }
    Ok(())
}

pub(super) fn validate_condition(
    condition: &CompiledConditionExpression,
) -> Result<(), CompiledAssetDecodeError> {
    match condition {
        CompiledConditionExpression::Call(call) => {
            validate_identifier("condition function", &call.function)?;
            for argument in &call.args {
                validate_argument(argument)?;
            }
        }
        CompiledConditionExpression::And(expressions) => {
            if expressions.is_empty() {
                return Err(malformed(
                    "condition and group must not be empty".to_owned(),
                ));
            }
            for expression in expressions {
                validate_condition(expression)?;
            }
        }
        CompiledConditionExpression::Or(expressions) => {
            if expressions.is_empty() {
                return Err(malformed("condition or group must not be empty".to_owned()));
            }
            for expression in expressions {
                validate_condition(expression)?;
            }
        }
        CompiledConditionExpression::Not(expression) => validate_condition(expression)?,
    }
    Ok(())
}

pub(super) fn validate_argument(
    argument: &CompiledArgument,
) -> Result<(), CompiledAssetDecodeError> {
    match argument {
        CompiledArgument::Identifier(value) => validate_identifier("argument identifier", value),
        CompiledArgument::Value(value) => validate_scalar("float scalar", value),
    }
}

pub(super) fn validate_effect(
    function: &str,
    args: &[CompiledArgument],
) -> Result<(), CompiledAssetDecodeError> {
    validate_identifier("effect function", function)?;
    for argument in args {
        validate_argument(argument)?;
    }
    Ok(())
}

pub(super) fn validate_reason_value(
    value: &CompiledAvailabilityReasonArgValue,
) -> Result<(), CompiledAssetDecodeError> {
    if let CompiledAvailabilityReasonArgValue::Literal(scalar) = value {
        validate_scalar("availability reason float literal", scalar)?;
    }
    Ok(())
}

pub(super) fn validate_non_empty(
    field: &'static str,
    value: &str,
) -> Result<(), CompiledAssetDecodeError> {
    if value.is_empty() {
        return Err(malformed(format!("{field} must not be empty")));
    }
    Ok(())
}

pub(super) fn validate_value(value: &Value) -> Result<(), CompiledAssetDecodeError> {
    match value {
        Value::Scalar(scalar) => validate_scalar("float scalar", scalar),
        Value::Array(scalars) => {
            for scalar in scalars {
                validate_scalar("float scalar", scalar)?;
            }
            Ok(())
        }
    }
}

pub(super) fn validate_span(span: &SourceSpan) -> Result<(), CompiledAssetDecodeError> {
    if span.end.is_some_and(|end| end < span.start) {
        return Err(malformed("source span end precedes span start".to_owned()));
    }
    Ok(())
}

pub(super) fn validate_choice_echo(
    dialogue: &CompiledDialogue,
    echo: &CompiledChoiceEcho,
) -> Result<(), CompiledAssetDecodeError> {
    let CompiledChoiceEcho::ExplicitLine(line_id) = echo else {
        return Ok(());
    };
    if dialogue
        .line_lookup
        .as_slice()
        .binary_search_by(|entry| entry.id.as_str().cmp(line_id.as_str()))
        .is_ok()
    {
        return Ok(());
    }
    Err(malformed(format!(
        "choice echo references unknown line id `{line_id}`"
    )))
}

fn validate_identifier(field: &'static str, value: &str) -> Result<(), CompiledAssetDecodeError> {
    ensure_identifier_like(field, value)
}

fn validate_scalar(
    field: &'static str,
    value: &ScalarValue,
) -> Result<(), CompiledAssetDecodeError> {
    if let ScalarValue::Float(value) = value
        && !value.is_finite()
    {
        return Err(malformed(format!("{field} must be finite")));
    }
    Ok(())
}
