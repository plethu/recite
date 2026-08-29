use std::collections::BTreeMap;

use super::super::diagnostics::{INVALID_TYPE_REFERENCE, MALFORMED_SHAPE};
use super::super::raw::RawValue;
use super::numeric::finite_f64_literal;
use crate::schema::{
    AvailabilityReasonArgBinding, ParameterDefinition, ProjectSchema, SchemaLiteralValue,
    SchemaTypeDefinition, SchemaTypeRef, schema_diagnostic,
};
use crate::{Diagnostic, DiagnosticArgumentValue, SourceSpan};

pub(super) fn literal_string_binding(
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    condition_name: &str,
    reason_param: &ParameterDefinition,
    value: &str,
    span: SourceSpan,
) -> Option<AvailabilityReasonArgBinding> {
    match &reason_param.type_ref {
        SchemaTypeRef::String
        | SchemaTypeRef::Speaker
        | SchemaTypeRef::Enum(_)
        | SchemaTypeRef::Registry(_) => {
            validate_string_domain(
                diagnostics,
                schema,
                condition_name,
                reason_param,
                value,
                span,
            )?;
            Some(AvailabilityReasonArgBinding::Literal(
                SchemaLiteralValue::String(value.to_owned()),
            ))
        }
        _ => {
            diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-availability-binding-string-type",
                format!(
                    "condition '{condition_name}' availability_reason argument '{}' expects {}, but got string literal",
                    reason_param.name,
                    type_ref_name(&reason_param.type_ref)
                ),
                span,
                [
                    (
                        "condition",
                        DiagnosticArgumentValue::String(condition_name.to_owned()),
                    ),
                    (
                        "argument",
                        DiagnosticArgumentValue::String(reason_param.name.clone()),
                    ),
                    (
                        "expected",
                        DiagnosticArgumentValue::String(type_ref_name(&reason_param.type_ref)),
                    ),
                ],
            ));
            None
        }
    }
}

pub(super) fn literal_non_string_binding(
    diagnostics: &mut Vec<Diagnostic>,
    condition_name: &str,
    reason_param: &ParameterDefinition,
    value: RawValue,
    span: SourceSpan,
) -> Option<AvailabilityReasonArgBinding> {
    match (&reason_param.type_ref, value) {
        (SchemaTypeRef::Bool, RawValue::Bool(value)) => {
            Some(AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::Bool(value)))
        }
        (SchemaTypeRef::Int, RawValue::Number(number)) => number.parse().ok().map_or_else(
            || {
                diagnostics.push(schema_diagnostic(
                    MALFORMED_SHAPE,
                    "diagnostic-schema-001-availability-binding-int",
                    format!(
                        "condition '{condition_name}' availability_reason argument '{}' expects int, but got non-integer number",
                        reason_param.name
                    ),
                    span,
                    [
                        (
                            "condition",
                            DiagnosticArgumentValue::String(condition_name.to_owned()),
                        ),
                        (
                            "argument",
                            DiagnosticArgumentValue::String(reason_param.name.clone()),
                        ),
                    ],
                ));
                None
            },
            |value| Some(AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::Int(value))),
        ),
        (SchemaTypeRef::Float, RawValue::Number(number)) => finite_f64_literal(
            diagnostics,
            &format!(
                "condition '{condition_name}' availability_reason argument '{}'",
                reason_param.name
            ),
            number,
            span,
        )
        .map(SchemaLiteralValue::Float)
        .map(AvailabilityReasonArgBinding::Literal),
        (type_ref, value) => {
            diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-availability-binding-literal-type",
                format!(
                    "condition '{condition_name}' availability_reason argument '{}' expects {}, but got {} literal",
                    reason_param.name,
                    type_ref_name(type_ref),
                    literal_kind(&value)
                ),
                span,
                [
                    (
                        "condition",
                        DiagnosticArgumentValue::String(condition_name.to_owned()),
                    ),
                    (
                        "argument",
                        DiagnosticArgumentValue::String(reason_param.name.clone()),
                    ),
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

pub(super) fn validate_string_domain(
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    condition_name: &str,
    reason_param: &ParameterDefinition,
    value: &str,
    span: SourceSpan,
) -> Option<()> {
    let known = match &reason_param.type_ref {
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
        "diagnostic-schema-001-availability-binding-unknown-value",
        format!(
            "condition '{condition_name}' availability_reason argument '{}' uses unknown {} value '{}'",
            reason_param.name,
            type_ref_name(&reason_param.type_ref),
            value
        ),
        span,
        [
            (
                "condition",
                DiagnosticArgumentValue::String(condition_name.to_owned()),
            ),
            (
                "argument",
                DiagnosticArgumentValue::String(reason_param.name.clone()),
            ),
            (
                "expected",
                DiagnosticArgumentValue::String(type_ref_name(&reason_param.type_ref)),
            ),
            (
                "value",
                DiagnosticArgumentValue::String(value.to_owned()),
            ),
        ],
    ));
    None
}

pub(super) fn literal_kind(value: &RawValue) -> &'static str {
    match value {
        RawValue::Null => "null",
        RawValue::Bool(_) => "bool",
        RawValue::Number(_) => "number",
        RawValue::String(_) => "string",
        RawValue::Array(_) => "array",
        RawValue::Object(_) => "object",
    }
}

pub(super) fn type_ref_name(type_ref: &SchemaTypeRef) -> String {
    match type_ref {
        SchemaTypeRef::String => "string".to_owned(),
        SchemaTypeRef::Symbol => "symbol".to_owned(),
        SchemaTypeRef::Int => "int".to_owned(),
        SchemaTypeRef::Float => "float".to_owned(),
        SchemaTypeRef::Bool => "bool".to_owned(),
        SchemaTypeRef::Speaker => "speaker".to_owned(),
        SchemaTypeRef::Enum(name) => format!("enum:{name}"),
        SchemaTypeRef::Registry(name) => format!("registry:{name}"),
        SchemaTypeRef::Array(inner) => format!("array:{}", type_ref_name(inner)),
    }
}

pub(super) fn lower_condition_param_binding(
    diagnostics: &mut Vec<Diagnostic>,
    condition_name: &str,
    condition_params_by_name: &BTreeMap<&str, &ParameterDefinition>,
    reason_param: &ParameterDefinition,
    condition_param_name: &str,
    span: SourceSpan,
) -> Option<AvailabilityReasonArgBinding> {
    let Some(condition_param) = condition_params_by_name.get(condition_param_name) else {
        diagnostics.push(schema_diagnostic(
            INVALID_TYPE_REFERENCE,
            "diagnostic-schema-004-unknown-condition-param",
            format!(
                "condition '{condition_name}' availability_reason references unknown condition parameter '{condition_param_name}'"
            ),
            span,
            [
                (
                    "condition",
                    DiagnosticArgumentValue::String(condition_name.to_owned()),
                ),
                (
                    "condition_param",
                    DiagnosticArgumentValue::String(condition_param_name.to_owned()),
                ),
            ],
        ));
        return None;
    };

    if condition_param.type_ref != reason_param.type_ref {
        diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-availability-binding-type-mismatch",
            format!(
                "condition '{condition_name}' availability_reason argument '{}' expects {}, but condition parameter '{}' has {}",
                reason_param.name,
                type_ref_name(&reason_param.type_ref),
                condition_param_name,
                type_ref_name(&condition_param.type_ref)
            ),
            span,
            [
                (
                    "condition",
                    DiagnosticArgumentValue::String(condition_name.to_owned()),
                ),
                (
                    "argument",
                    DiagnosticArgumentValue::String(reason_param.name.clone()),
                ),
                (
                    "expected",
                    DiagnosticArgumentValue::String(type_ref_name(&reason_param.type_ref)),
                ),
                (
                    "condition_param",
                    DiagnosticArgumentValue::String(condition_param_name.to_owned()),
                ),
                (
                    "actual",
                    DiagnosticArgumentValue::String(type_ref_name(&condition_param.type_ref)),
                ),
            ],
        ));
        return None;
    }

    Some(AvailabilityReasonArgBinding::ConditionParam(
        condition_param_name.to_owned(),
    ))
}
