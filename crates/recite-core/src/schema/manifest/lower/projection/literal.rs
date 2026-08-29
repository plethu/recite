use super::super::super::diagnostics::MALFORMED_SHAPE;
use super::super::super::raw::RawValue;
use super::super::numeric::finite_f64_literal;
use super::reference::type_ref_name;
use crate::schema::schema_diagnostic;
use crate::schema::{ProjectSchema, SchemaLiteralValue, SchemaTypeDefinition, SchemaTypeRef};
use crate::{Diagnostic, DiagnosticArgumentValue, SourceSpan};

pub(super) fn lower_literal_for_type(
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    owner: &str,
    type_ref: &SchemaTypeRef,
    value: RawValue,
    span: SourceSpan,
) -> Option<SchemaLiteralValue> {
    match (type_ref, value) {
        (SchemaTypeRef::String | SchemaTypeRef::Speaker, RawValue::String(value)) => {
            validate_string_value(diagnostics, schema, owner, type_ref, &value, span)?;
            Some(SchemaLiteralValue::String(value))
        }
        (SchemaTypeRef::Enum(_) | SchemaTypeRef::Registry(_), RawValue::String(value)) => {
            validate_string_value(diagnostics, schema, owner, type_ref, &value, span)?;
            Some(SchemaLiteralValue::String(value))
        }
        (SchemaTypeRef::Int, RawValue::Number(number)) => number.parse().ok().map_or_else(
            || {
                diagnostics.push(schema_diagnostic(
                    MALFORMED_SHAPE,
                    "diagnostic-schema-001-projection-literal-int",
                    format!("{owner} expects int, but got non-integer number"),
                    span,
                    [("owner", DiagnosticArgumentValue::String(owner.to_owned()))],
                ));
                None
            },
            |value| Some(SchemaLiteralValue::Int(value)),
        ),
        (SchemaTypeRef::Float, RawValue::Number(number)) => {
            finite_f64_literal(diagnostics, owner, number, span).map(SchemaLiteralValue::Float)
        }
        (SchemaTypeRef::Bool, RawValue::Bool(value)) => Some(SchemaLiteralValue::Bool(value)),
        (type_ref, value) => {
            diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-projection-literal-type",
                format!(
                    "{owner} expects {}, but got {} literal",
                    type_ref_name(type_ref),
                    literal_kind(&value)
                ),
                span,
                [
                    ("owner", DiagnosticArgumentValue::String(owner.to_owned())),
                    (
                        "expected",
                        DiagnosticArgumentValue::String(type_ref_name(type_ref)),
                    ),
                    (
                        "actual",
                        DiagnosticArgumentValue::String(literal_kind(&value).to_owned()),
                    ),
                ],
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

    diagnostics.push(schema_diagnostic(
        MALFORMED_SHAPE,
        "diagnostic-schema-001-projection-literal-unknown",
        format!(
            "{owner} uses unknown {} value '{}'",
            type_ref_name(type_ref),
            value
        ),
        span,
        [
            ("owner", DiagnosticArgumentValue::String(owner.to_owned())),
            (
                "expected",
                DiagnosticArgumentValue::String(type_ref_name(type_ref)),
            ),
            ("value", DiagnosticArgumentValue::String(value.to_owned())),
        ],
    ));
    None
}

fn literal_kind(value: &RawValue) -> &'static str {
    match value {
        RawValue::Null => "null",
        RawValue::Bool(_) => "bool",
        RawValue::Number(_) => "number",
        RawValue::String(_) => "string",
        RawValue::Array(_) => "array",
        RawValue::Object(_) => "object",
    }
}
