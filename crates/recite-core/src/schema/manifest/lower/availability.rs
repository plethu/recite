use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::super::diagnostics::{DUPLICATE_DEFINITION, INVALID_TYPE_REFERENCE, MALFORMED_SHAPE};
use super::super::raw::{Named, RawAvailabilityReasonDefinition};
use super::super::spans::ManifestSpans;
use super::super::validate::{
    PendingTypeReference, duplicate_definition, validate_manifest_name, validate_non_empty_string,
};
use super::functions::{PendingConditionAvailabilityReasonMapping, lower_params};
use crate::schema::{
    AvailabilityReasonArgBinding, AvailabilityReasonDefinition, ConditionAvailabilityReasonMapping,
    ConditionReturnType, ParameterDefinition, ProjectSchema, SchemaLiteralValue, SchemaTypeRef,
};
use crate::{AvailabilityReasonId, Diagnostic, SourceSpan, extract_placeholder_names};

pub(super) fn lower_availability_reasons(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawAvailabilityReasonDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(
            diagnostics,
            "availability reason id",
            &entry.name,
            name_span.clone(),
        ) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "availability reason", &entry.name, name_span);
            continue;
        }

        let template_span = spans.next_value_span(file, source, &entry.value.template);
        validate_non_empty_string(
            diagnostics,
            "availability reason template",
            &entry.value.template,
            template_span.clone(),
        );
        let params = lower_params(
            file,
            source,
            spans,
            diagnostics,
            &format!("availability reason '{}'", entry.name),
            &entry.value.params,
            pending_type_refs,
        );
        validate_template_placeholders(
            diagnostics,
            &entry.name,
            &entry.value.template,
            &params,
            template_span,
        );

        if let Some(origin) = &entry.value.origin {
            let origin_span = spans.next_value_span(file, source, origin);
            validate_non_empty_string(
                diagnostics,
                "availability reason origin",
                origin,
                origin_span,
            );
        }

        let Ok(reason_id) = AvailabilityReasonId::new(entry.name) else {
            continue;
        };
        schema.availability_reasons.insert(
            reason_id,
            AvailabilityReasonDefinition {
                template: entry.value.template,
                params,
                origin: entry.value.origin,
            },
        );
    }
}

pub(super) fn validate_condition_availability_reason_mappings(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    mappings: Vec<PendingConditionAvailabilityReasonMapping>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for pending in mappings {
        let condition_span = spans.next_key_span(file, source, &pending.condition);
        let Some(condition) = schema.conditions.get(&pending.condition).cloned() else {
            continue;
        };

        if condition.returns != ConditionReturnType::Bool {
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "condition '{}' availability_reason mapping is only allowed on bool-returning conditions",
                    pending.condition
                ),
                condition_span.clone(),
            ));
        }

        let reason_span = spans.next_value_span(file, source, &pending.raw.reason);
        if !validate_manifest_name(
            diagnostics,
            "availability reason id",
            &pending.raw.reason,
            reason_span.clone(),
        ) {
            continue;
        }
        let Ok(reason_id) = AvailabilityReasonId::new(pending.raw.reason.clone()) else {
            continue;
        };
        let Some(reason) = schema.availability_reasons.get(&reason_id).cloned() else {
            diagnostics.push(Diagnostic::error(
                INVALID_TYPE_REFERENCE,
                format!(
                    "condition '{}' availability_reason references unknown reason '{}'",
                    pending.condition, pending.raw.reason
                ),
                reason_span.clone(),
            ));
            continue;
        };

        let mut lowerer = MappingLowerer::new(
            file,
            source,
            spans,
            diagnostics,
            schema,
            &pending.condition,
            &condition.params,
        );
        if let Some(args) =
            lower_mapping_args(&mut lowerer, &reason.params, pending.raw.args, reason_span)
            && let Some(condition) = schema.conditions.get_mut(&pending.condition)
        {
            condition.availability_reason = Some(ConditionAvailabilityReasonMapping {
                reason: reason_id,
                args,
            });
        }
    }
}

fn validate_template_placeholders(
    diagnostics: &mut Vec<Diagnostic>,
    reason: &str,
    template: &str,
    params: &[ParameterDefinition],
    span: SourceSpan,
) {
    let placeholders = match extract_placeholder_names(template) {
        Ok(placeholders) => placeholders,
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "availability reason '{reason}' template has invalid placeholder syntax: {}",
                    error.message()
                ),
                span,
            ));
            return;
        }
    };
    let param_names = params
        .iter()
        .map(|param| param.name.as_str())
        .collect::<BTreeSet<_>>();

    for placeholder in &placeholders {
        if !param_names.contains(placeholder.as_str()) {
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "availability reason '{reason}' template references unknown parameter '{placeholder}'"
                ),
                span.clone(),
            ));
        }
    }
    for param in params {
        if !placeholders.contains(&param.name) {
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "availability reason '{reason}' parameter '{}' is not used in its template",
                    param.name
                ),
                span.clone(),
            ));
        }
    }
}

struct MappingLowerer<'a, 'b> {
    file: &'a str,
    source: &'a str,
    spans: &'a mut ManifestSpans,
    diagnostics: &'a mut Vec<Diagnostic>,
    schema: &'a ProjectSchema,
    condition_name: &'a str,
    condition_params_by_name: BTreeMap<&'b str, &'b ParameterDefinition>,
}

impl<'a, 'b> MappingLowerer<'a, 'b> {
    fn new(
        file: &'a str,
        source: &'a str,
        spans: &'a mut ManifestSpans,
        diagnostics: &'a mut Vec<Diagnostic>,
        schema: &'a ProjectSchema,
        condition_name: &'a str,
        condition_params: &'b [ParameterDefinition],
    ) -> Self {
        Self {
            file,
            source,
            spans,
            diagnostics,
            schema,
            condition_name,
            condition_params_by_name: condition_params
                .iter()
                .map(|param| (param.name.as_str(), param))
                .collect(),
        }
    }
}

fn lower_mapping_args(
    lowerer: &mut MappingLowerer<'_, '_>,
    reason_params: &[ParameterDefinition],
    raw_args: Vec<Named<Value>>,
    mapping_span: SourceSpan,
) -> Option<BTreeMap<String, AvailabilityReasonArgBinding>> {
    let reason_params_by_name = reason_params
        .iter()
        .map(|param| (param.name.as_str(), param))
        .collect::<BTreeMap<_, _>>();
    let mut seen = BTreeSet::new();
    let mut lowered = BTreeMap::new();
    let mut valid = true;

    for raw_arg in raw_args {
        let arg_span = lowerer
            .spans
            .next_key_span(lowerer.file, lowerer.source, &raw_arg.name);
        if !seen.insert(raw_arg.name.clone()) {
            lowerer.diagnostics.push(Diagnostic::error(
                DUPLICATE_DEFINITION,
                format!(
                    "condition '{}' availability_reason repeats argument '{}'",
                    lowerer.condition_name, raw_arg.name
                ),
                arg_span,
            ));
            valid = false;
            continue;
        }
        let Some(reason_param) = reason_params_by_name.get(raw_arg.name.as_str()) else {
            lowerer.diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "condition '{}' availability_reason binds unknown reason parameter '{}'",
                    lowerer.condition_name, raw_arg.name
                ),
                arg_span,
            ));
            valid = false;
            continue;
        };

        let Some(binding) = lower_arg_binding(lowerer, reason_param, raw_arg.value, arg_span)
        else {
            valid = false;
            continue;
        };
        lowered.insert(raw_arg.name, binding);
    }

    for reason_param in reason_params {
        if !lowered.contains_key(&reason_param.name) {
            lowerer.diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "condition '{}' availability_reason is missing argument '{}'",
                    lowerer.condition_name, reason_param.name
                ),
                mapping_span.clone(),
            ));
            valid = false;
        }
    }

    valid.then_some(lowered)
}

fn lower_arg_binding(
    lowerer: &mut MappingLowerer<'_, '_>,
    reason_param: &ParameterDefinition,
    value: Value,
    fallback_span: SourceSpan,
) -> Option<AvailabilityReasonArgBinding> {
    if let Some(value) = value.as_str() {
        let value_span = lowerer
            .spans
            .next_value_span(lowerer.file, lowerer.source, value);
        if let Some(condition_param_name) = value.strip_prefix('$') {
            return lower_condition_param_binding(
                lowerer.diagnostics,
                lowerer.condition_name,
                &lowerer.condition_params_by_name,
                reason_param,
                condition_param_name,
                value_span,
            );
        }
        return literal_string_binding(
            lowerer.diagnostics,
            lowerer.schema,
            lowerer.condition_name,
            reason_param,
            value,
            value_span,
        );
    }

    literal_non_string_binding(
        lowerer.diagnostics,
        lowerer.condition_name,
        reason_param,
        value,
        fallback_span,
    )
}

fn lower_condition_param_binding(
    diagnostics: &mut Vec<Diagnostic>,
    condition_name: &str,
    condition_params_by_name: &BTreeMap<&str, &ParameterDefinition>,
    reason_param: &ParameterDefinition,
    condition_param_name: &str,
    span: SourceSpan,
) -> Option<AvailabilityReasonArgBinding> {
    let Some(condition_param) = condition_params_by_name.get(condition_param_name) else {
        diagnostics.push(Diagnostic::error(
            INVALID_TYPE_REFERENCE,
            format!(
                "condition '{condition_name}' availability_reason references unknown condition parameter '{condition_param_name}'"
            ),
            span,
        ));
        return None;
    };

    if condition_param.type_ref != reason_param.type_ref {
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!(
                "condition '{condition_name}' availability_reason argument '{}' expects {}, but condition parameter '{}' has {}",
                reason_param.name,
                type_ref_name(&reason_param.type_ref),
                condition_param_name,
                type_ref_name(&condition_param.type_ref)
            ),
            span,
        ));
        return None;
    }

    Some(AvailabilityReasonArgBinding::ConditionParam(
        condition_param_name.to_owned(),
    ))
}

fn literal_string_binding(
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    condition_name: &str,
    reason_param: &ParameterDefinition,
    value: &str,
    span: SourceSpan,
) -> Option<AvailabilityReasonArgBinding> {
    match &reason_param.type_ref {
        SchemaTypeRef::String | SchemaTypeRef::Speaker => {
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
        SchemaTypeRef::Enum(_) | SchemaTypeRef::Registry(_) => {
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
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "condition '{condition_name}' availability_reason argument '{}' expects {}, but got string literal",
                    reason_param.name,
                    type_ref_name(&reason_param.type_ref)
                ),
                span,
            ));
            None
        }
    }
}

fn literal_non_string_binding(
    diagnostics: &mut Vec<Diagnostic>,
    condition_name: &str,
    reason_param: &ParameterDefinition,
    value: Value,
    span: SourceSpan,
) -> Option<AvailabilityReasonArgBinding> {
    match (&reason_param.type_ref, value) {
        (SchemaTypeRef::Bool, Value::Bool(value)) => Some(AvailabilityReasonArgBinding::Literal(
            SchemaLiteralValue::Bool(value),
        )),
        (SchemaTypeRef::Int, Value::Number(number)) => {
            number.as_i64().map_or_else(
                || {
                    diagnostics.push(Diagnostic::error(
                        MALFORMED_SHAPE,
                        format!(
                            "condition '{condition_name}' availability_reason argument '{}' expects int, but got non-integer number",
                            reason_param.name
                        ),
                        span,
                    ));
                    None
                },
                |value| {
                    Some(AvailabilityReasonArgBinding::Literal(
                        SchemaLiteralValue::Int(value),
                    ))
                },
            )
        }
        (SchemaTypeRef::Float, Value::Number(number)) => Some(
            AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::Float(number.to_string())),
        ),
        (type_ref, value) => {
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "condition '{condition_name}' availability_reason argument '{}' expects {}, but got {} literal",
                    reason_param.name,
                    type_ref_name(type_ref),
                    literal_kind(&value)
                ),
                span,
            ));
            None
        }
    }
}

fn validate_string_domain(
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
                    crate::schema::SchemaTypeDefinition::Enum(definition) => {
                        definition.values.contains(value)
                    }
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
            "condition '{condition_name}' availability_reason argument '{}' uses unknown {} value '{}'",
            reason_param.name,
            type_ref_name(&reason_param.type_ref),
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

fn type_ref_name(type_ref: &SchemaTypeRef) -> String {
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
