use serde_json::Value;

use super::super::super::diagnostics::MALFORMED_SHAPE;
use super::reference::type_ref_name;
use crate::schema::{ProjectSchema, SchemaLiteralValue, SchemaTypeDefinition, SchemaTypeRef};
use crate::{Diagnostic, SourceSpan};

pub(super) fn lower_literal_for_type(
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    owner: &str,
    type_ref: &SchemaTypeRef,
    value: Value,
    span: SourceSpan,
) -> Option<SchemaLiteralValue> {
    match (type_ref, value) {
        (SchemaTypeRef::String | SchemaTypeRef::Speaker, Value::String(value)) => {
            validate_string_value(diagnostics, schema, owner, type_ref, &value, span)?;
            Some(SchemaLiteralValue::String(value))
        }
        (SchemaTypeRef::Enum(_) | SchemaTypeRef::Registry(_), Value::String(value)) => {
            validate_string_value(diagnostics, schema, owner, type_ref, &value, span)?;
            Some(SchemaLiteralValue::String(value))
        }
        (SchemaTypeRef::Int, Value::Number(number)) => number.as_i64().map_or_else(
            || {
                diagnostics.push(Diagnostic::error(
                    MALFORMED_SHAPE,
                    format!("{owner} expects int, but got non-integer number"),
                    span,
                ));
                None
            },
            |value| Some(SchemaLiteralValue::Int(value)),
        ),
        (SchemaTypeRef::Float, Value::Number(number)) => {
            Some(SchemaLiteralValue::Float(number.to_string()))
        }
        (SchemaTypeRef::Bool, Value::Bool(value)) => Some(SchemaLiteralValue::Bool(value)),
        (type_ref, value) => {
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "{owner} expects {}, but got {} literal",
                    type_ref_name(type_ref),
                    literal_kind(&value)
                ),
                span,
            ));
            None
        }
    }
}

fn validate_string_value(
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    owner: &str,
    type_ref: &SchemaTypeRef,
    value: &str,
    span: SourceSpan,
) -> Option<()> {
    let known = match type_ref {
        SchemaTypeRef::Speaker => schema.speakers.contains_key(value),
        SchemaTypeRef::Enum(name) => {
            schema
                .types
                .get(name)
                .is_none_or(|definition| match definition {
                    SchemaTypeDefinition::Enum(definition) => definition.values.contains(value),
                })
        }
        SchemaTypeRef::Registry(name) => schema
            .registries
            .get(name)
            .is_none_or(|definition| definition.values.contains(value)),
        _ => true,
    };

    if known {
        return Some(());
    }

    diagnostics.push(Diagnostic::error(
        MALFORMED_SHAPE,
        format!(
            "{owner} uses unknown {} value '{}'",
            type_ref_name(type_ref),
            value
        ),
        span,
    ));
    None
}

fn literal_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
