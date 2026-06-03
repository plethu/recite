use std::collections::BTreeSet;

use super::super::diagnostics::{DUPLICATE_DEFINITION, INVALID_TYPE_REFERENCE, MALFORMED_SHAPE};
use super::super::raw::{
    Named, RawConditionAvailabilityReasonMapping, RawConditionDefinition, RawEffectDefinition,
    RawParameterDefinition,
};
use super::super::spans::ManifestSpans;
use super::super::validate::{
    PendingTypeReference, duplicate_definition, parse_effect_mode, parse_enum_return,
    validate_manifest_name, validate_non_empty_string,
};
use super::types::lower_type_reference;
use crate::Diagnostic;
use crate::schema::{
    ConditionDefinition, ConditionReturnType, EffectDefinition, ParameterDefinition, ProjectSchema,
    SchemaTypeRef,
};

pub(super) struct PendingConditionAvailabilityReasonMapping {
    pub(super) condition: String,
    pub(super) raw: RawConditionAvailabilityReasonMapping,
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
        let name_span = spans.next_key_span(file, source, &entry.name);
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

        let params = lower_params(
            file,
            source,
            spans,
            diagnostics,
            &format!("condition '{}'", entry.name),
            &entry.value.params,
            pending_refs.type_refs,
        );
        let returns = match entry.value.returns.as_deref() {
            None | Some("bool") => ConditionReturnType::Bool,
            Some(value) => {
                let return_span = spans.next_value_span(file, source, value);
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
                        diagnostics.push(Diagnostic::error(
                            INVALID_TYPE_REFERENCE,
                            format!(
                                "condition '{}' has invalid return type '{}'",
                                entry.name, value
                            ),
                            return_span,
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
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(diagnostics, "effect name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "effect", &entry.name, name_span);
            continue;
        }

        let mut modes = BTreeSet::new();
        for mode in &entry.value.modes {
            let mode_span = spans.next_value_span(file, source, mode);
            let Some(effect_mode) = parse_effect_mode(mode) else {
                diagnostics.push(Diagnostic::error(
                    MALFORMED_SHAPE,
                    format!("effect '{}' uses unsupported mode '{}'", entry.name, mode),
                    mode_span,
                ));
                continue;
            };

            if !modes.insert(effect_mode) {
                diagnostics.push(Diagnostic::error(
                    DUPLICATE_DEFINITION,
                    format!("effect '{}' repeats mode '{}'", entry.name, mode),
                    mode_span,
                ));
            }
        }

        let params = lower_params(
            file,
            source,
            spans,
            diagnostics,
            &format!("effect '{}'", entry.name),
            &entry.value.params,
            pending_type_refs,
        );
        schema
            .effects
            .insert(entry.name, EffectDefinition { modes, params });
    }
}

pub(super) fn lower_params(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    owner: &str,
    params: &[RawParameterDefinition],
    pending_type_refs: &mut Vec<PendingTypeReference>,
) -> Vec<ParameterDefinition> {
    let mut seen = BTreeSet::new();
    params
        .iter()
        .map(|param| {
            let name_span = spans.next_value_span(file, source, &param.name);
            if validate_non_empty_string(
                diagnostics,
                "parameter name",
                &param.name,
                name_span.clone(),
            ) {
                validate_manifest_name(
                    diagnostics,
                    "parameter name",
                    &param.name,
                    name_span.clone(),
                );
            }
            if !seen.insert(param.name.clone()) {
                diagnostics.push(Diagnostic::error(
                    DUPLICATE_DEFINITION,
                    format!("{owner} repeats parameter '{}'", param.name),
                    name_span,
                ));
            }

            let (mut type_ref, type_ref_span, type_ref_is_valid) = lower_type_reference(
                file,
                source,
                spans,
                diagnostics,
                &param.type_ref,
                format!(
                    "parameter '{}' has invalid type reference '{}'",
                    param.name, param.type_ref
                ),
            );
            let type_ref_is_valid = type_ref_is_valid
                && validate_parameter_type_ref(
                    diagnostics,
                    owner,
                    param,
                    &mut type_ref,
                    &type_ref_span,
                );
            if type_ref_is_valid {
                pending_type_refs.push(PendingTypeReference {
                    owner: format!("{owner} parameter '{}'", param.name),
                    type_ref: type_ref.clone(),
                    span: type_ref_span,
                });
            }

            ParameterDefinition {
                name: param.name.clone(),
                type_ref,
            }
        })
        .collect()
}

fn validate_parameter_type_ref(
    diagnostics: &mut Vec<Diagnostic>,
    owner: &str,
    param: &RawParameterDefinition,
    type_ref: &mut SchemaTypeRef,
    type_ref_span: &crate::SourceSpan,
) -> bool {
    if !matches!(type_ref, SchemaTypeRef::Symbol) {
        return true;
    }

    diagnostics.push(Diagnostic::error(
        INVALID_TYPE_REFERENCE,
        format!(
            "{owner} parameter '{}' uses metadata-only type reference 'symbol'",
            param.name
        ),
        type_ref_span.clone(),
    ));
    *type_ref = SchemaTypeRef::String;
    false
}
