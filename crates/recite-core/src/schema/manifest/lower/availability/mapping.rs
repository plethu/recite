use std::collections::{BTreeMap, BTreeSet};

use super::super::super::diagnostics::{DUPLICATE_DEFINITION, MALFORMED_SHAPE};
use super::super::super::raw::{Named, RawValue};
use super::super::super::spans::ManifestSpans;
use super::super::ManifestSourceFormat;
use super::super::availability_bindings::{
    literal_non_string_binding, literal_string_binding, lower_condition_param_binding,
};
use crate::schema::{
    AvailabilityReasonArgBinding, ParameterDefinition, ProjectSchema, schema_diagnostic,
};
use crate::{Diagnostic, DiagnosticArgumentValue, SourceSpan};

pub(super) struct MappingLowerer<'a, 'b> {
    pub(super) file: &'a str,
    pub(super) source: &'a str,
    pub(super) spans: &'a mut ManifestSpans,
    pub(super) diagnostics: &'a mut Vec<Diagnostic>,
    pub(super) schema: &'a ProjectSchema,
    pub(super) condition_name: &'a str,
    pub(super) mapping_path: &'a [String],
    pub(super) condition_params_by_name: BTreeMap<&'b str, &'b ParameterDefinition>,
    pub(super) format: ManifestSourceFormat,
}

pub(super) fn lower_mapping_args(
    lowerer: &mut MappingLowerer<'_, '_>,
    reason_params: &[ParameterDefinition],
    raw_args: Vec<Named<RawValue>>,
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
        let mut arg_path = lowerer.mapping_path.to_vec();
        arg_path.extend([
            "availability_reason".to_owned(),
            "args".to_owned(),
            raw_arg.name.clone(),
        ]);
        let arg_span =
            lowerer
                .spans
                .key_span_at(lowerer.file, lowerer.source, &arg_path, &raw_arg.name);
        if !seen.insert(raw_arg.name.clone()) {
            lowerer.diagnostics.push(schema_diagnostic(
                DUPLICATE_DEFINITION,
                "diagnostic-schema-003-availability-argument",
                format!(
                    "condition '{}' availability_reason repeats argument '{}'",
                    lowerer.condition_name, raw_arg.name
                ),
                arg_span,
                [
                    (
                        "condition",
                        DiagnosticArgumentValue::String(lowerer.condition_name.to_owned()),
                    ),
                    (
                        "argument",
                        DiagnosticArgumentValue::String(raw_arg.name.clone()),
                    ),
                ],
            ));
            valid = false;
            continue;
        }
        let Some(reason_param) = reason_params_by_name.get(raw_arg.name.as_str()) else {
            lowerer.diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-availability-unknown-reason-param",
                format!(
                    "condition '{}' availability_reason binds unknown reason parameter '{}'",
                    lowerer.condition_name, raw_arg.name
                ),
                arg_span,
                [
                    (
                        "condition",
                        DiagnosticArgumentValue::String(lowerer.condition_name.to_owned()),
                    ),
                    (
                        "argument",
                        DiagnosticArgumentValue::String(raw_arg.name.clone()),
                    ),
                ],
            ));
            valid = false;
            continue;
        };

        let Some(binding) =
            lower_arg_binding(lowerer, reason_param, raw_arg.value, arg_span, &arg_path)
        else {
            valid = false;
            continue;
        };
        lowered.insert(raw_arg.name, binding);
    }

    for reason_param in reason_params {
        if !lowered.contains_key(&reason_param.name) {
            lowerer.diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-availability-missing-reason-arg",
                format!(
                    "condition '{}' availability_reason is missing argument '{}'",
                    lowerer.condition_name, reason_param.name
                ),
                mapping_span.clone(),
                [
                    (
                        "condition",
                        DiagnosticArgumentValue::String(lowerer.condition_name.to_owned()),
                    ),
                    (
                        "argument",
                        DiagnosticArgumentValue::String(reason_param.name.clone()),
                    ),
                ],
            ));
            valid = false;
        }
    }

    valid.then_some(lowered)
}

fn lower_arg_binding(
    lowerer: &mut MappingLowerer<'_, '_>,
    reason_param: &ParameterDefinition,
    value: RawValue,
    fallback_span: SourceSpan,
    arg_path: &[String],
) -> Option<AvailabilityReasonArgBinding> {
    if let RawValue::Object(fields) = &value {
        if lowerer.format == ManifestSourceFormat::Json {
            lowerer.diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-availability-tagged-only-toml",
                "tagged availability reason arguments are only supported in TOML",
                fallback_span,
                std::iter::empty::<(String, DiagnosticArgumentValue)>(),
            ));
            return None;
        }
        let Some(kind) = fields.get("kind").and_then(RawValue::as_str) else {
            lowerer.diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-availability-tag-missing-kind",
                "availability reason argument tag must contain kind",
                fallback_span,
                std::iter::empty::<(String, DiagnosticArgumentValue)>(),
            ));
            return None;
        };
        match kind {
            "binding" => {
                let Some(name) = fields.get("name").and_then(RawValue::as_str) else {
                    lowerer.diagnostics.push(schema_diagnostic(
                        MALFORMED_SHAPE,
                        "diagnostic-schema-001-availability-binding-missing-name",
                        "availability reason binding must contain name",
                        fallback_span,
                        std::iter::empty::<(String, DiagnosticArgumentValue)>(),
                    ));
                    return None;
                };
                let mut name_path = arg_path.to_vec();
                name_path.push("name".to_owned());
                return lower_condition_param_binding(
                    lowerer.diagnostics,
                    lowerer.condition_name,
                    &lowerer.condition_params_by_name,
                    reason_param,
                    name,
                    lowerer
                        .spans
                        .value_span_at(lowerer.file, lowerer.source, &name_path, name),
                );
            }
            "literal" => {
                let Some(literal) = fields.get("value") else {
                    lowerer.diagnostics.push(schema_diagnostic(
                        MALFORMED_SHAPE,
                        "diagnostic-schema-001-availability-literal-missing-value",
                        "availability reason literal must contain value",
                        fallback_span,
                        std::iter::empty::<(String, DiagnosticArgumentValue)>(),
                    ));
                    return None;
                };
                let mut value_path = arg_path.to_vec();
                value_path.push("value".to_owned());
                let literal_span = literal.as_str().map_or_else(
                    || fallback_span.clone(),
                    |value| {
                        lowerer.spans.value_span_at(
                            lowerer.file,
                            lowerer.source,
                            &value_path,
                            value,
                        )
                    },
                );
                return lower_explicit_literal(
                    lowerer,
                    reason_param,
                    literal.clone(),
                    literal_span,
                );
            }
            _ => {
                lowerer.diagnostics.push(schema_diagnostic(
                    MALFORMED_SHAPE,
                    "diagnostic-schema-001-availability-tag-kind",
                    format!("unsupported availability reason argument kind '{kind}'"),
                    fallback_span,
                    [("kind", DiagnosticArgumentValue::String(kind.to_owned()))],
                ));
                return None;
            }
        }
    }
    if let Some(value) = value.as_str() {
        let value_span = lowerer
            .spans
            .value_span_at(lowerer.file, lowerer.source, arg_path, value);
        if lowerer.format == ManifestSourceFormat::Json && value.starts_with("$$") {
            return literal_string_binding(
                lowerer.diagnostics,
                lowerer.schema,
                lowerer.condition_name,
                reason_param,
                &value[1..],
                value_span,
            );
        }
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

fn lower_explicit_literal(
    lowerer: &mut MappingLowerer<'_, '_>,
    reason_param: &ParameterDefinition,
    value: RawValue,
    span: SourceSpan,
) -> Option<AvailabilityReasonArgBinding> {
    if let Some(value) = value.as_str() {
        return literal_string_binding(
            lowerer.diagnostics,
            lowerer.schema,
            lowerer.condition_name,
            reason_param,
            value,
            span,
        );
    }
    literal_non_string_binding(
        lowerer.diagnostics,
        lowerer.condition_name,
        reason_param,
        value,
        span,
    )
}
