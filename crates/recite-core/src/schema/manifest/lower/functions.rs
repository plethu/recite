use std::collections::BTreeSet;

use super::super::raw::{
    Named, RawConditionAvailabilityReasonMapping, RawConditionDefinition, RawEffectDefinition,
};
use super::super::spans::ManifestSpans;
use super::super::validate::{
    PendingTypeReference, duplicate_definition, parse_effect_mode, parse_enum_return,
    validate_manifest_name,
};
pub(super) use super::parameters::lower_params_at;
use crate::schema::{
    ConditionDefinition, ConditionReturnType, EffectDefinition, ProjectSchema, SchemaTypeRef,
    schema_diagnostic,
};
use crate::{Diagnostic, DiagnosticArgumentValue};

pub(super) struct PendingConditionAvailabilityReasonMapping {
    pub(super) condition: String,
    pub(super) raw: RawConditionAvailabilityReasonMapping,
    pub(super) path: Vec<String>,
}

pub(super) struct FunctionPendingReferences<'a> {
    pub(super) type_refs: &'a mut Vec<PendingTypeReference>,
    pub(super) availability_reason_mappings: &'a mut Vec<PendingConditionAvailabilityReasonMapping>,
}

pub(super) fn lower_conditions(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawConditionDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
    pending_refs: FunctionPendingReferences<'_>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let entry_path = vec!["conditions".to_owned(), entry.name.clone()];
        let name_span = spans.key_span_at(file, source, &entry_path, &entry.name);
        if !validate_manifest_name(
            diagnostics,
            "condition name",
            &entry.name,
            name_span.clone(),
        ) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "condition", &entry.name, name_span);
            continue;
        }

        let params = lower_params_at(
            file,
            source,
            spans,
            diagnostics,
            &format!("condition '{}'", entry.name),
            &entry.value.params,
            pending_refs.type_refs,
            &entry_path,
        );
        let returns = match entry.value.returns.as_deref() {
            None | Some("bool") => ConditionReturnType::Bool,
            Some(value) => {
                let mut return_path = entry_path.clone();
                return_path.push("returns".to_owned());
                let return_span = spans.value_span_at(file, source, &return_path, value);
                match parse_enum_return(value) {
                    Some(name) => {
                        pending_refs.type_refs.push(PendingTypeReference {
                            owner: format!("condition '{}' return type", entry.name),
                            type_ref: SchemaTypeRef::Enum(name.clone()),
                            span: return_span,
                        });
                        ConditionReturnType::Enum(name)
                    }
                    None => {
                        diagnostics.push(schema_diagnostic(
                            super::super::diagnostics::INVALID_TYPE_REFERENCE,
                            "diagnostic-schema-004-invalid-condition-return",
                            format!(
                                "condition '{}' has invalid return type '{}'",
                                entry.name, value
                            ),
                            return_span,
                            [
                                (
                                    "condition",
                                    DiagnosticArgumentValue::String(entry.name.clone()),
                                ),
                                (
                                    "return_type",
                                    DiagnosticArgumentValue::String(value.to_owned()),
                                ),
                            ],
                        ));
                        ConditionReturnType::Bool
                    }
                }
            }
        };

        if let Some(mapping) = entry.value.availability_reason {
            pending_refs.availability_reason_mappings.push(
                PendingConditionAvailabilityReasonMapping {
                    condition: entry.name.clone(),
                    raw: mapping,
                    path: entry_path.clone(),
                },
            );
        }

        schema.conditions.insert(
            entry.name,
            ConditionDefinition {
                params,
                returns,
                availability_reason: None,
            },
        );
    }
}

pub(super) fn lower_effects(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawEffectDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let entry_path = vec!["effects".to_owned(), entry.name.clone()];
        let name_span = spans.key_span_at(file, source, &entry_path, &entry.name);
        if !validate_manifest_name(diagnostics, "effect name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "effect", &entry.name, name_span);
            continue;
        }

        let mut modes = BTreeSet::new();
        for (index, mode) in entry.value.modes.iter().enumerate() {
            let mut mode_path = entry_path.clone();
            mode_path.extend(["modes".to_owned(), format!("[{index}]")]);
            let mode_span = spans.value_span_at(file, source, &mode_path, mode);
            let Some(effect_mode) = parse_effect_mode(mode) else {
                diagnostics.push(schema_diagnostic(
                    super::super::diagnostics::MALFORMED_SHAPE,
                    "diagnostic-schema-001-effect-mode",
                    format!("effect '{}' uses unsupported mode '{}'", entry.name, mode),
                    mode_span,
                    [
                        (
                            "effect",
                            DiagnosticArgumentValue::String(entry.name.clone()),
                        ),
                        ("mode", DiagnosticArgumentValue::String(mode.clone())),
                    ],
                ));
                continue;
            };

            if !modes.insert(effect_mode) {
                diagnostics.push(schema_diagnostic(
                    super::super::diagnostics::DUPLICATE_DEFINITION,
                    "diagnostic-schema-003-effect-mode",
                    format!("effect '{}' repeats mode '{}'", entry.name, mode),
                    mode_span,
                    [
                        (
                            "effect",
                            DiagnosticArgumentValue::String(entry.name.clone()),
                        ),
                        ("mode", DiagnosticArgumentValue::String(mode.clone())),
                    ],
                ));
            }
        }

        let params = lower_params_at(
            file,
            source,
            spans,
            diagnostics,
            &format!("effect '{}'", entry.name),
            &entry.value.params,
            pending_type_refs,
            &entry_path,
        );
        schema
            .effects
            .insert(entry.name, EffectDefinition { modes, params });
    }
}
