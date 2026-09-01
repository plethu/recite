use std::collections::BTreeSet;

mod mapping;
mod templates;

use self::mapping::{MappingLowerer, lower_mapping_args};
use self::templates::validate_template_placeholders;
use super::super::diagnostics::{INVALID_TYPE_REFERENCE, MALFORMED_SHAPE};
use super::super::raw::{Named, RawAvailabilityReasonDefinition};
use super::super::spans::ManifestSpans;
use super::super::validate::{
    PendingTypeReference, duplicate_definition, validate_manifest_name, validate_non_empty_string,
};
use super::LoweringContext;
use super::ManifestSourceFormat;
use super::functions::{PendingConditionAvailabilityReasonMapping, lower_params_at};
use super::producer::{ProvenanceLocation, lower_origin};
use crate::schema::{
    AvailabilityReasonDefinition, ConditionAvailabilityReasonMapping, ConditionReturnType,
    ProjectSchema, schema_diagnostic,
};
use crate::{AvailabilityReasonId, Diagnostic, DiagnosticArgumentValue};

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
        let entry_path = vec!["availability_reasons".to_owned(), entry.name.clone()];
        let name_span = spans.key_span_at(file, source, &entry_path, &entry.name);
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

        let mut template_path = entry_path.clone();
        template_path.push("template".to_owned());
        let template_span =
            spans.value_span_at(file, source, &template_path, &entry.value.template);
        validate_non_empty_string(
            diagnostics,
            "availability reason template",
            &entry.value.template,
            template_span.clone(),
        );
        let params = lower_params_at(
            &mut LoweringContext::new(file, source, spans, diagnostics),
            &format!("availability reason '{}'", entry.name),
            &entry.value.params,
            pending_type_refs,
            &entry_path,
        );
        validate_template_placeholders(
            diagnostics,
            &entry.name,
            &entry.value.template,
            &params,
            template_span,
        );

        let origin_path = {
            let mut path = entry_path.clone();
            path.push("origin".to_owned());
            path
        };
        let origin = lower_origin(
            &mut LoweringContext::new(file, source, spans, diagnostics),
            entry.value.origin,
            ProvenanceLocation {
                owner: &format!("availability reason '{}'", entry.name),
                span: name_span.clone(),
                path: &origin_path,
            },
        );

        let Ok(reason_id) = AvailabilityReasonId::new(entry.name) else {
            continue;
        };
        schema.availability_reasons.insert(
            reason_id,
            AvailabilityReasonDefinition {
                template: entry.value.template,
                params,
                origin,
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
    format: ManifestSourceFormat,
) {
    for pending in mappings {
        let condition_span = spans.key_span_at(file, source, &pending.path, &pending.condition);
        let Some(condition) = schema.conditions.get(&pending.condition).cloned() else {
            continue;
        };

        if condition.returns != ConditionReturnType::Bool {
            diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-availability-non-bool-mapping",
                format!(
                    "condition '{}' availability_reason mapping is only allowed on bool-returning conditions",
                    pending.condition
                ),
                condition_span.clone(),
                [(
                    "condition",
                    DiagnosticArgumentValue::String(pending.condition.clone()),
                )],
            ));
        }

        let mut reason_path = pending.path.clone();
        reason_path.extend(["availability_reason".to_owned(), "reason".to_owned()]);
        let reason_span = spans.value_span_at(file, source, &reason_path, &pending.raw.reason);
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
            diagnostics.push(schema_diagnostic(
                INVALID_TYPE_REFERENCE,
                "diagnostic-schema-004-unknown-availability-reason",
                format!(
                    "condition '{}' availability_reason references unknown reason '{}'",
                    pending.condition, pending.raw.reason
                ),
                reason_span.clone(),
                [
                    (
                        "condition",
                        DiagnosticArgumentValue::String(pending.condition.clone()),
                    ),
                    (
                        "reason",
                        DiagnosticArgumentValue::String(pending.raw.reason.clone()),
                    ),
                ],
            ));
            continue;
        };

        let mut lowerer = MappingLowerer {
            file,
            source,
            spans,
            diagnostics,
            schema,
            condition_name: &pending.condition,
            mapping_path: &pending.path,
            condition_params_by_name: condition
                .params
                .iter()
                .map(|param| (param.name.as_str(), param))
                .collect(),
            format,
        };
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
