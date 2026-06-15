use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::super::diagnostics::{DUPLICATE_DEFINITION, INVALID_TYPE_REFERENCE, MALFORMED_SHAPE};
use super::super::raw::{
    Named, RawMetadataOccurrence, RawPresentationAffordanceFieldDefinition,
    RawPresentationAffordanceFieldSource, RawPresentationAffordanceOutputDefinition,
    RawPresentationLabelArgDefinition, RawPresentationLabelDefinition,
    RawPresentationProjectorDefinition, RawProjectionInput, RawProjectionInputRef,
    RawProjectionInputSource, RawProjectionQueryDefinition, RawProjectionQueryFunctionDefinition,
    RawProjectionSelector,
};
use super::super::spans::ManifestSpans;
use super::super::validate::{
    PendingTypeReference, duplicate_definition, parse_metadata_target, validate_manifest_name,
    validate_non_empty_string,
};
use super::functions::lower_params;
use super::types::lower_type_reference;
use crate::schema::{
    MetadataDefinition, MetadataOccurrence, MetadataTarget, ParameterDefinition,
    PresentationAffordanceFieldDefinition, PresentationAffordanceFieldSource,
    PresentationAffordanceOutputDefinition, PresentationLabelArgDefinition,
    PresentationLabelDefinition, ProjectSchema, ProjectionInput, ProjectionInputRef,
    ProjectionOutputTarget, ProjectionQueryDefinition, ProjectionQueryFunctionDefinition,
    SchemaLiteralValue, SchemaPresentationProjectorDefinition, SchemaProjectionInputSource,
    SchemaProjectionSelector, SchemaTypeRef,
};
use crate::{AvailabilityReasonId, Diagnostic, SourceSpan, extract_placeholder_names};

pub(super) fn lower_projection_queries(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawProjectionQueryFunctionDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(
            diagnostics,
            "projection query function name",
            &entry.name,
            name_span.clone(),
        ) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(
                diagnostics,
                "projection query function",
                &entry.name,
                name_span,
            );
            continue;
        }

        let params = lower_params(
            file,
            source,
            spans,
            diagnostics,
            &format!("projection query function '{}'", entry.name),
            &entry.value.params,
            pending_type_refs,
        );
        let (returns, returns_span, returns_valid) = lower_type_reference(
            file,
            source,
            spans,
            diagnostics,
            &entry.value.returns,
            format!(
                "projection query function '{}' has invalid return type '{}'",
                entry.name, entry.value.returns
            ),
        );
        if returns_valid {
            pending_type_refs.push(PendingTypeReference {
                owner: format!("projection query function '{}' return type", entry.name),
                type_ref: returns.clone(),
                span: returns_span,
            });
        }
        if let Some(max_calls) = entry.value.max_calls_per_event
            && max_calls == 0
        {
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "projection query function '{}' max_calls_per_event must be greater than zero",
                    entry.name
                ),
                name_span,
            ));
        }

        schema.projection_queries.insert(
            entry.name,
            ProjectionQueryFunctionDefinition {
                params,
                returns,
                max_calls_per_event: entry.value.max_calls_per_event,
            },
        );
    }
}

pub(super) fn lower_presentation_projectors(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawPresentationProjectorDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) {
    let mut seen = BTreeSet::new();
    let mut seen_label_ids = BTreeSet::new();
    for entry in entries {
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(diagnostics, "projector id", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(
                diagnostics,
                "presentation projector",
                &entry.name,
                name_span,
            );
            continue;
        }

        let Some(candidates) = lower_selector(
            file,
            source,
            spans,
            diagnostics,
            schema,
            &entry.name,
            entry.value.candidates,
        ) else {
            continue;
        };
        let inputs = lower_inputs(
            file,
            source,
            spans,
            diagnostics,
            schema,
            &entry.name,
            &candidates,
            entry.value.inputs,
            pending_type_refs,
        );
        let queries = lower_projector_queries(
            file,
            source,
            spans,
            diagnostics,
            schema,
            &entry.name,
            &inputs,
            entry.value.queries,
        );
        let outputs = lower_outputs(
            file,
            source,
            spans,
            diagnostics,
            schema,
            &entry.name,
            &inputs,
            &queries,
            entry.value.outputs,
            &mut seen_label_ids,
            pending_type_refs,
        );

        schema.presentation_projectors.insert(
            entry.name,
            SchemaPresentationProjectorDefinition {
                candidates,
                inputs,
                queries,
                outputs,
            },
        );
    }
}

fn lower_selector(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    raw: RawProjectionSelector,
) -> Option<SchemaProjectionSelector> {
    match raw {
        RawProjectionSelector::RuntimeEvent { event } => {
            let span = spans.next_value_span(file, source, &event);
            validate_non_empty_string(diagnostics, "projection runtime event kind", &event, span);
            Some(SchemaProjectionSelector::RuntimeEvent { kind: event })
        }
        RawProjectionSelector::MetadataKey { target, key } => {
            let target =
                lower_projector_metadata_target(file, source, spans, diagnostics, &target)?;
            validate_metadata_key_target(
                diagnostics,
                schema,
                projector,
                &key,
                target,
                spans.next_value_span(file, source, &key),
            );
            Some(SchemaProjectionSelector::MetadataKey { target, key })
        }
        RawProjectionSelector::MetadataSet {
            target,
            required_keys,
        } => {
            let target =
                lower_projector_metadata_target(file, source, spans, diagnostics, &target)?;
            let mut seen_keys = BTreeSet::new();
            for key in &required_keys {
                let key_span = spans.next_value_span(file, source, key);
                validate_metadata_key_target(
                    diagnostics,
                    schema,
                    projector,
                    key,
                    target,
                    key_span.clone(),
                );
                if !seen_keys.insert(key.clone()) {
                    diagnostics.push(Diagnostic::error(
                        DUPLICATE_DEFINITION,
                        format!("projector '{projector}' repeats required metadata key '{key}'"),
                        key_span,
                    ));
                }
            }
            Some(SchemaProjectionSelector::MetadataSet {
                target,
                required_keys,
            })
        }
        RawProjectionSelector::AvailabilityReason { reason } => {
            let reason_span = spans.next_value_span(file, source, &reason);
            if !validate_manifest_name(
                diagnostics,
                "availability reason id",
                &reason,
                reason_span.clone(),
            ) {
                return None;
            }
            let Ok(reason_id) = AvailabilityReasonId::new(reason.clone()) else {
                return None;
            };
            if !schema.availability_reasons.contains_key(&reason_id) {
                diagnostics.push(Diagnostic::error(
                    INVALID_TYPE_REFERENCE,
                    format!(
                        "projector '{projector}' references unknown availability reason '{reason}'"
                    ),
                    reason_span,
                ));
            }
            Some(SchemaProjectionSelector::AvailabilityReason { reason_id })
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
fn lower_inputs(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    selector: &SchemaProjectionSelector,
    raw_inputs: Vec<RawProjectionInput>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) -> Vec<ProjectionInput> {
    let mut seen = BTreeSet::new();
    raw_inputs
        .into_iter()
        .map(|raw| {
            let name_span = spans.next_value_span(file, source, &raw.name);
            validate_manifest_name(
                diagnostics,
                "projection input name",
                &raw.name,
                name_span.clone(),
            );
            if !seen.insert(raw.name.clone()) {
                diagnostics.push(Diagnostic::error(
                    DUPLICATE_DEFINITION,
                    format!("projector '{projector}' repeats input '{}'", raw.name),
                    name_span,
                ));
            }

            let (type_ref, type_ref_span, type_ref_valid) = lower_type_reference(
                file,
                source,
                spans,
                diagnostics,
                &raw.type_ref,
                format!(
                    "projector '{projector}' input '{}' has invalid type reference '{}'",
                    raw.name, raw.type_ref
                ),
            );
            if type_ref_valid {
                pending_type_refs.push(PendingTypeReference {
                    owner: format!("projector '{projector}' input '{}'", raw.name),
                    type_ref: type_ref.clone(),
                    span: type_ref_span,
                });
            }

            let source = lower_input_source(
                file,
                source,
                spans,
                diagnostics,
                schema,
                projector,
                selector,
                &raw.name,
                &type_ref,
                raw.source,
            );
            ProjectionInput {
                name: raw.name,
                source,
                type_ref,
                required: raw.required,
            }
        })
        .collect()
}

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
fn lower_input_source(
    file: &str,
    source_text: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    selector: &SchemaProjectionSelector,
    input: &str,
    type_ref: &SchemaTypeRef,
    raw: RawProjectionInputSource,
) -> SchemaProjectionInputSource {
    match raw {
        RawProjectionInputSource::EventKind => SchemaProjectionInputSource::EventKind,
        RawProjectionInputSource::CandidateLineId => {
            validate_candidate_source(
                diagnostics,
                projector,
                input,
                selector,
                CandidateKind::Line,
                spans.next_key_span(file, source_text, input),
            );
            SchemaProjectionInputSource::CandidateLineId
        }
        RawProjectionInputSource::CandidateChoiceId => {
            validate_candidate_source(
                diagnostics,
                projector,
                input,
                selector,
                CandidateKind::Choice,
                spans.next_key_span(file, source_text, input),
            );
            SchemaProjectionInputSource::CandidateChoiceId
        }
        RawProjectionInputSource::CandidateEffectRequestId => {
            validate_candidate_source(
                diagnostics,
                projector,
                input,
                selector,
                CandidateKind::Effect,
                spans.next_key_span(file, source_text, input),
            );
            SchemaProjectionInputSource::CandidateEffectRequestId
        }
        RawProjectionInputSource::CandidateBlockId => {
            validate_candidate_source(
                diagnostics,
                projector,
                input,
                selector,
                CandidateKind::Block,
                spans.next_key_span(file, source_text, input),
            );
            SchemaProjectionInputSource::CandidateBlockId
        }
        RawProjectionInputSource::CandidateProject => {
            validate_candidate_source(
                diagnostics,
                projector,
                input,
                selector,
                CandidateKind::Project,
                spans.next_key_span(file, source_text, input),
            );
            SchemaProjectionInputSource::CandidateProject
        }
        RawProjectionInputSource::CandidateMetadata { key, occurrence } => {
            let key_span = spans.next_value_span(file, source_text, &key);
            let occurrence =
                lower_occurrence(diagnostics, projector, input, occurrence, key_span.clone());
            validate_candidate_metadata_source(
                diagnostics,
                schema,
                projector,
                input,
                selector,
                &key,
                &occurrence,
                type_ref,
                key_span,
            );
            SchemaProjectionInputSource::CandidateMetadata { key, occurrence }
        }
        RawProjectionInputSource::AvailabilityReasonArg { name } => {
            let name_span = spans.next_value_span(file, source_text, &name);
            validate_availability_reason_arg_source(
                diagnostics,
                schema,
                projector,
                input,
                selector,
                &name,
                type_ref,
                name_span,
            );
            SchemaProjectionInputSource::AvailabilityReasonArg { name }
        }
        RawProjectionInputSource::Literal { value } => {
            let literal = lower_literal_for_type(
                diagnostics,
                schema,
                &format!("projector '{projector}' input '{input}'"),
                type_ref,
                value,
                spans.next_key_span(file, source_text, input),
            )
            .unwrap_or_else(|| SchemaLiteralValue::String(String::new()));
            SchemaProjectionInputSource::Literal(literal)
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
fn lower_projector_queries(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    inputs: &[ProjectionInput],
    raw_queries: Vec<Named<RawProjectionQueryDefinition>>,
) -> BTreeMap<String, ProjectionQueryDefinition> {
    let input_types = inputs
        .iter()
        .map(|input| (input.name.as_str(), &input.type_ref))
        .collect::<BTreeMap<_, _>>();
    let mut query_types = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut lowered = BTreeMap::new();

    for raw_query in raw_queries {
        let query_span = spans.next_key_span(file, source, &raw_query.name);
        validate_manifest_name(
            diagnostics,
            "projection query name",
            &raw_query.name,
            query_span.clone(),
        );
        if !seen.insert(raw_query.name.clone()) {
            diagnostics.push(Diagnostic::error(
                DUPLICATE_DEFINITION,
                format!("projector '{projector}' repeats query '{}'", raw_query.name),
                query_span,
            ));
            continue;
        }
        let function_span = spans.next_value_span(file, source, &raw_query.value.function);
        let function = schema.projection_queries.get(&raw_query.value.function);
        let Some(function) = function else {
            diagnostics.push(Diagnostic::error(
                INVALID_TYPE_REFERENCE,
                format!(
                    "projector '{projector}' query '{}' references unknown projection query function '{}'",
                    raw_query.name, raw_query.value.function
                ),
                function_span,
            ));
            lowered.insert(
                raw_query.name,
                ProjectionQueryDefinition {
                    function: raw_query.value.function,
                    args: lower_input_refs(raw_query.value.args),
                },
            );
            continue;
        };

        let args = lower_and_validate_query_args(
            diagnostics,
            projector,
            &raw_query.name,
            &raw_query.value.function,
            &function.params,
            raw_query.value.args,
            &input_types,
            &query_types,
            function_span.clone(),
        );
        query_types.insert(raw_query.name.clone(), function.returns.clone());
        lowered.insert(
            raw_query.name,
            ProjectionQueryDefinition {
                function: raw_query.value.function,
                args,
            },
        );
    }

    lowered
}

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
fn lower_outputs(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    inputs: &[ProjectionInput],
    queries: &BTreeMap<String, ProjectionQueryDefinition>,
    raw_outputs: Vec<Named<RawPresentationAffordanceOutputDefinition>>,
    seen_label_ids: &mut BTreeSet<String>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) -> BTreeMap<String, PresentationAffordanceOutputDefinition> {
    let mut seen = BTreeSet::new();
    let mut lowered = BTreeMap::new();
    let input_types = inputs
        .iter()
        .map(|input| (input.name.as_str(), input.type_ref.clone()))
        .collect::<BTreeMap<_, _>>();
    let query_types = query_result_types(schema, queries);

    for raw_output in raw_outputs {
        let output_span = spans.next_key_span(file, source, &raw_output.name);
        validate_manifest_name(
            diagnostics,
            "projection output id",
            &raw_output.name,
            output_span.clone(),
        );
        if !seen.insert(raw_output.name.clone()) {
            diagnostics.push(Diagnostic::error(
                DUPLICATE_DEFINITION,
                format!(
                    "projector '{projector}' repeats output '{}'",
                    raw_output.name
                ),
                output_span,
            ));
            continue;
        }
        let target = lower_output_target(
            diagnostics,
            projector,
            &raw_output.name,
            &raw_output.value.target,
            spans.next_value_span(file, source, &raw_output.value.target),
        );
        let kind_span = spans.next_value_span(file, source, &raw_output.value.kind);
        validate_non_empty_string(
            diagnostics,
            "projection output kind",
            &raw_output.value.kind,
            kind_span,
        );
        let slot_span = spans.next_value_span(file, source, &raw_output.value.slot);
        validate_non_empty_string(
            diagnostics,
            "projection output slot",
            &raw_output.value.slot,
            slot_span,
        );
        let label = raw_output.value.label.map(|label| {
            lower_label(
                file,
                source,
                spans,
                diagnostics,
                projector,
                &raw_output.name,
                label,
                &input_types,
                &query_types,
                seen_label_ids,
                pending_type_refs,
            )
        });
        let fields = lower_fields(
            file,
            source,
            spans,
            diagnostics,
            projector,
            &raw_output.name,
            raw_output.value.fields,
            schema,
            &input_types,
            &query_types,
            pending_type_refs,
        );
        lowered.insert(
            raw_output.name,
            PresentationAffordanceOutputDefinition {
                target,
                kind: raw_output.value.kind,
                slot: raw_output.value.slot,
                label,
                fields,
            },
        );
    }

    lowered
}

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
fn lower_label(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    output: &str,
    raw_label: RawPresentationLabelDefinition,
    input_types: &BTreeMap<&str, SchemaTypeRef>,
    query_types: &BTreeMap<String, SchemaTypeRef>,
    seen_label_ids: &mut BTreeSet<String>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) -> PresentationLabelDefinition {
    let template_id_span = spans.next_value_span(file, source, &raw_label.template_id);
    validate_manifest_name(
        diagnostics,
        "presentation label template id",
        &raw_label.template_id,
        template_id_span,
    );
    if !seen_label_ids.insert(raw_label.template_id.clone()) {
        diagnostics.push(Diagnostic::error(
            DUPLICATE_DEFINITION,
            format!(
                "duplicate presentation label template id '{}'",
                raw_label.template_id
            ),
            spans.next_value_span(file, source, &raw_label.template_id),
        ));
    }
    let source_text_span = spans.next_value_span(file, source, &raw_label.source_text);
    validate_non_empty_string(
        diagnostics,
        "presentation label source text",
        &raw_label.source_text,
        source_text_span.clone(),
    );
    let args = lower_label_args(
        file,
        source,
        spans,
        diagnostics,
        projector,
        output,
        raw_label.args,
        input_types,
        query_types,
        pending_type_refs,
    );
    validate_label_placeholders(
        diagnostics,
        projector,
        output,
        &raw_label.template_id,
        &raw_label.source_text,
        &args,
        source_text_span,
    );
    PresentationLabelDefinition {
        template_id: raw_label.template_id,
        source_text: raw_label.source_text,
        args,
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
fn lower_label_args(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    output: &str,
    raw_args: Vec<Named<RawPresentationLabelArgDefinition>>,
    input_types: &BTreeMap<&str, SchemaTypeRef>,
    query_types: &BTreeMap<String, SchemaTypeRef>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) -> BTreeMap<String, PresentationLabelArgDefinition> {
    let mut seen = BTreeSet::new();
    let mut lowered = BTreeMap::new();
    for raw_arg in raw_args {
        let arg_span = spans.next_key_span(file, source, &raw_arg.name);
        validate_manifest_name(
            diagnostics,
            "presentation label argument name",
            &raw_arg.name,
            arg_span.clone(),
        );
        if !seen.insert(raw_arg.name.clone()) {
            diagnostics.push(Diagnostic::error(
                DUPLICATE_DEFINITION,
                format!(
                    "projector '{projector}' output '{output}' repeats label argument '{}'",
                    raw_arg.name
                ),
                arg_span,
            ));
            continue;
        }
        let type_ref = lower_output_type_ref(
            file,
            source,
            spans,
            diagnostics,
            projector,
            output,
            &raw_arg.name,
            &raw_arg.value.type_ref,
            pending_type_refs,
        );
        let source_ref = lower_input_ref(raw_arg.value.source);
        validate_ref_type(
            diagnostics,
            projector,
            &format!("output '{output}' label argument '{}'", raw_arg.name),
            &source_ref,
            &type_ref,
            input_types,
            query_types,
            arg_span,
        );
        lowered.insert(
            raw_arg.name,
            PresentationLabelArgDefinition {
                source: source_ref,
                type_ref,
            },
        );
    }
    lowered
}

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
fn lower_fields(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    output: &str,
    raw_fields: Vec<Named<RawPresentationAffordanceFieldDefinition>>,
    schema: &ProjectSchema,
    input_types: &BTreeMap<&str, SchemaTypeRef>,
    query_types: &BTreeMap<String, SchemaTypeRef>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) -> BTreeMap<String, PresentationAffordanceFieldDefinition> {
    let mut seen = BTreeSet::new();
    let mut lowered = BTreeMap::new();
    for raw_field in raw_fields {
        let field_span = spans.next_key_span(file, source, &raw_field.name);
        validate_manifest_name(
            diagnostics,
            "projection output field name",
            &raw_field.name,
            field_span.clone(),
        );
        if !seen.insert(raw_field.name.clone()) {
            diagnostics.push(Diagnostic::error(
                DUPLICATE_DEFINITION,
                format!(
                    "projector '{projector}' output '{output}' repeats field '{}'",
                    raw_field.name
                ),
                field_span,
            ));
            continue;
        }
        let type_ref = lower_output_type_ref(
            file,
            source,
            spans,
            diagnostics,
            projector,
            output,
            &raw_field.name,
            &raw_field.value.type_ref,
            pending_type_refs,
        );
        let source = lower_field_source(
            diagnostics,
            schema,
            projector,
            output,
            &raw_field.name,
            raw_field.value.source,
            &type_ref,
            input_types,
            query_types,
            field_span,
        );
        lowered.insert(
            raw_field.name,
            PresentationAffordanceFieldDefinition { source, type_ref },
        );
    }
    lowered
}

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
fn lower_output_type_ref(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    output: &str,
    name: &str,
    raw_type: &str,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) -> SchemaTypeRef {
    let (type_ref, type_ref_span, valid) = lower_type_reference(
        file,
        source,
        spans,
        diagnostics,
        raw_type,
        format!(
            "projector '{projector}' output '{output}' binding '{name}' has invalid type reference '{raw_type}'"
        ),
    );
    if valid {
        pending_type_refs.push(PendingTypeReference {
            owner: format!("projector '{projector}' output '{output}' binding '{name}'"),
            type_ref: type_ref.clone(),
            span: type_ref_span,
        });
    }
    type_ref
}

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
fn lower_and_validate_query_args(
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    query: &str,
    function: &str,
    params: &[ParameterDefinition],
    raw_args: Vec<RawProjectionInputRef>,
    input_types: &BTreeMap<&str, &SchemaTypeRef>,
    query_types: &BTreeMap<String, SchemaTypeRef>,
    span: SourceSpan,
) -> Vec<ProjectionInputRef> {
    if raw_args.len() != params.len() {
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!(
                "projector '{projector}' query '{query}' passes {} args to projection query function '{function}', expected {}",
                raw_args.len(),
                params.len()
            ),
            span.clone(),
        ));
    }
    raw_args
        .into_iter()
        .enumerate()
        .map(|(index, raw)| {
            let input_ref = lower_input_ref(raw);
            if let Some(param) = params.get(index) {
                validate_ref_type(
                    diagnostics,
                    projector,
                    &format!("query '{query}' argument '{}'", param.name),
                    &input_ref,
                    &param.type_ref,
                    input_types,
                    query_types,
                    span.clone(),
                );
            }
            input_ref
        })
        .collect()
}

fn lower_input_refs(raw_refs: Vec<RawProjectionInputRef>) -> Vec<ProjectionInputRef> {
    raw_refs.into_iter().map(lower_input_ref).collect()
}

fn lower_input_ref(raw: RawProjectionInputRef) -> ProjectionInputRef {
    match raw {
        RawProjectionInputRef::Input { input } => ProjectionInputRef::Input { name: input },
        RawProjectionInputRef::QueryResult { query_result } => {
            ProjectionInputRef::QueryResult { name: query_result }
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared schema and type context"
)]
fn lower_field_source(
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    output: &str,
    field: &str,
    raw: RawPresentationAffordanceFieldSource,
    type_ref: &SchemaTypeRef,
    input_types: &BTreeMap<&str, SchemaTypeRef>,
    query_types: &BTreeMap<String, SchemaTypeRef>,
    span: SourceSpan,
) -> PresentationAffordanceFieldSource {
    match raw {
        RawPresentationAffordanceFieldSource::Input { name } => {
            let input_ref = ProjectionInputRef::Input { name: name.clone() };
            validate_ref_type(
                diagnostics,
                projector,
                &format!("output '{output}' field '{field}'"),
                &input_ref,
                type_ref,
                input_types,
                query_types,
                span,
            );
            PresentationAffordanceFieldSource::Input { name }
        }
        RawPresentationAffordanceFieldSource::QueryResult { name } => {
            let input_ref = ProjectionInputRef::QueryResult { name: name.clone() };
            validate_ref_type(
                diagnostics,
                projector,
                &format!("output '{output}' field '{field}'"),
                &input_ref,
                type_ref,
                input_types,
                query_types,
                span,
            );
            PresentationAffordanceFieldSource::QueryResult { name }
        }
        RawPresentationAffordanceFieldSource::Literal { value } => {
            let literal = lower_literal_for_type(
                diagnostics,
                schema,
                &format!("projector '{projector}' output '{output}' field '{field}'"),
                type_ref,
                value,
                span,
            )
            .unwrap_or_else(|| SchemaLiteralValue::String(String::new()));
            PresentationAffordanceFieldSource::Literal(literal)
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "projection reference validation carries owner, expected type, and source maps"
)]
fn validate_ref_type<T>(
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    owner: &str,
    input_ref: &ProjectionInputRef,
    expected: &SchemaTypeRef,
    input_types: &BTreeMap<&str, T>,
    query_types: &BTreeMap<String, SchemaTypeRef>,
    span: SourceSpan,
) where
    T: std::borrow::Borrow<SchemaTypeRef>,
{
    let actual = match input_ref {
        ProjectionInputRef::Input { name } => input_types
            .get(name.as_str())
            .map(std::borrow::Borrow::borrow),
        ProjectionInputRef::QueryResult { name } => query_types.get(name),
    };
    let Some(actual) = actual else {
        diagnostics.push(Diagnostic::error(
            INVALID_TYPE_REFERENCE,
            format!(
                "projector '{projector}' {owner} references unknown {}",
                ref_name(input_ref)
            ),
            span,
        ));
        return;
    };
    if actual != expected {
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!(
                "projector '{projector}' {owner} expects {}, but {} has {}",
                type_ref_name(expected),
                ref_name(input_ref),
                type_ref_name(actual)
            ),
            span,
        ));
    }
}

fn query_result_types(
    schema: &ProjectSchema,
    queries: &BTreeMap<String, ProjectionQueryDefinition>,
) -> BTreeMap<String, SchemaTypeRef> {
    queries
        .iter()
        .filter_map(|(name, query)| {
            schema
                .projection_queries
                .get(&query.function)
                .map(|function| (name.clone(), function.returns.clone()))
        })
        .collect()
}

fn validate_label_placeholders(
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    output: &str,
    template_id: &str,
    source_text: &str,
    args: &BTreeMap<String, PresentationLabelArgDefinition>,
    span: SourceSpan,
) {
    let placeholders = match extract_placeholder_names(source_text) {
        Ok(placeholders) => placeholders,
        Err(error) => {
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "projector '{projector}' output '{output}' presentation label '{template_id}' has invalid placeholder syntax: {}",
                    error.message()
                ),
                span,
            ));
            return;
        }
    };
    for placeholder in &placeholders {
        if !args.contains_key(placeholder) {
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "projector '{projector}' output '{output}' presentation label '{template_id}' references unknown argument '{placeholder}'"
                ),
                span.clone(),
            ));
        }
    }
    for arg in args.keys() {
        if !placeholders.contains(arg) {
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "projector '{projector}' output '{output}' presentation label '{template_id}' argument '{arg}' is not used in its template"
                ),
                span.clone(),
            ));
        }
    }
}

fn lower_projector_metadata_target(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    raw: &str,
) -> Option<MetadataTarget> {
    let span = spans.next_value_span(file, source, raw);
    parse_metadata_target(raw).or_else(|| {
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!("presentation projector uses unsupported metadata target '{raw}'"),
            span,
        ));
        None
    })
}

fn validate_metadata_key_target(
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    key: &str,
    target: MetadataTarget,
    span: SourceSpan,
) -> Option<MetadataDefinition> {
    let Some(metadata) = schema.metadata.get(key) else {
        diagnostics.push(Diagnostic::error(
            INVALID_TYPE_REFERENCE,
            format!("projector '{projector}' references unknown metadata key '{key}'"),
            span,
        ));
        return None;
    };
    if !metadata.targets.contains(&target) {
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!(
                "projector '{projector}' references metadata key '{key}' on unsupported target '{}'",
                metadata_target_name(target)
            ),
            span,
        ));
    }
    Some(metadata.clone())
}

#[expect(
    clippy::too_many_arguments,
    reason = "projection metadata validation carries selector, schema, type, and span context"
)]
fn validate_candidate_metadata_source(
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    input: &str,
    selector: &SchemaProjectionSelector,
    key: &str,
    occurrence: &MetadataOccurrence,
    type_ref: &SchemaTypeRef,
    span: SourceSpan,
) {
    let Some(target) = selector_metadata_target(selector) else {
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!("projector '{projector}' input '{input}' reads candidate metadata but its selector has no metadata target"),
            span,
        ));
        return;
    };
    let Some(metadata) =
        validate_metadata_key_target(diagnostics, schema, projector, key, target, span.clone())
    else {
        return;
    };
    if !metadata.repeatable && !matches!(occurrence, MetadataOccurrence::Only) {
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!("projector '{projector}' input '{input}' uses repeated occurrence '{}' for non-repeatable metadata key '{key}'", occurrence_name(occurrence)),
            span.clone(),
        ));
    }
    match occurrence {
        MetadataOccurrence::All => {
            let SchemaTypeRef::Array(inner) = type_ref else {
                diagnostics.push(Diagnostic::error(
                    MALFORMED_SHAPE,
                    format!("projector '{projector}' input '{input}' uses occurrence 'all' but has non-array type {}", type_ref_name(type_ref)),
                    span,
                ));
                return;
            };
            if **inner != metadata.type_ref {
                diagnostics.push(Diagnostic::error(
                    MALFORMED_SHAPE,
                    format!("projector '{projector}' input '{input}' expects {}, but metadata key '{key}' has {}", type_ref_name(type_ref), type_ref_name(&metadata.type_ref)),
                    span,
                ));
            }
        }
        _ if matches!(type_ref, SchemaTypeRef::Array(_)) => diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!("projector '{projector}' input '{input}' uses array type {} without occurrence 'all'", type_ref_name(type_ref)),
            span,
        )),
        _ if *type_ref != metadata.type_ref => diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!("projector '{projector}' input '{input}' expects {}, but metadata key '{key}' has {}", type_ref_name(type_ref), type_ref_name(&metadata.type_ref)),
            span,
        )),
        _ => {}
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "availability projection validation carries selector, schema, type, and span context"
)]
fn validate_availability_reason_arg_source(
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    input: &str,
    selector: &SchemaProjectionSelector,
    name: &str,
    type_ref: &SchemaTypeRef,
    span: SourceSpan,
) {
    let SchemaProjectionSelector::AvailabilityReason { reason_id } = selector else {
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!("projector '{projector}' input '{input}' reads an availability reason argument but its selector is not availability_reason"),
            span,
        ));
        return;
    };
    let Some(reason) = schema.availability_reasons.get(reason_id) else {
        return;
    };
    let Some(param) = reason.params.iter().find(|param| param.name == name) else {
        diagnostics.push(Diagnostic::error(
            INVALID_TYPE_REFERENCE,
            format!(
                "projector '{projector}' input '{input}' references unknown availability reason argument '{name}'"
            ),
            span,
        ));
        return;
    };
    if &param.type_ref != type_ref {
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!(
                "projector '{projector}' input '{input}' expects {}, but availability reason argument '{name}' has {}",
                type_ref_name(type_ref),
                type_ref_name(&param.type_ref)
            ),
            span,
        ));
    }
}

#[derive(Clone, Copy)]
enum CandidateKind {
    Line,
    Choice,
    Effect,
    Block,
    Project,
}

fn validate_candidate_source(
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    input: &str,
    selector: &SchemaProjectionSelector,
    candidate: CandidateKind,
    span: SourceSpan,
) {
    let valid = match (selector, candidate) {
        (SchemaProjectionSelector::MetadataKey { target, .. }, CandidateKind::Line)
        | (SchemaProjectionSelector::MetadataSet { target, .. }, CandidateKind::Line) => {
            *target == MetadataTarget::Line
        }
        (SchemaProjectionSelector::MetadataKey { target, .. }, CandidateKind::Choice)
        | (SchemaProjectionSelector::MetadataSet { target, .. }, CandidateKind::Choice) => {
            *target == MetadataTarget::Choice
        }
        (SchemaProjectionSelector::MetadataKey { target, .. }, CandidateKind::Block)
        | (SchemaProjectionSelector::MetadataSet { target, .. }, CandidateKind::Block) => {
            *target == MetadataTarget::Block
        }
        (SchemaProjectionSelector::MetadataKey { target, .. }, CandidateKind::Project)
        | (SchemaProjectionSelector::MetadataSet { target, .. }, CandidateKind::Project) => {
            *target == MetadataTarget::Project
        }
        (SchemaProjectionSelector::RuntimeEvent { kind }, CandidateKind::Effect) => {
            kind == "effect"
        }
        (SchemaProjectionSelector::RuntimeEvent { .. }, _) => true,
        _ => false,
    };
    if !valid {
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!(
                "projector '{projector}' input '{input}' uses an incompatible candidate id source"
            ),
            span,
        ));
    }
}

fn lower_occurrence(
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    input: &str,
    raw: Option<RawMetadataOccurrence>,
    span: SourceSpan,
) -> MetadataOccurrence {
    match raw {
        None => MetadataOccurrence::Only,
        Some(RawMetadataOccurrence::Named(name)) if name == "only" => MetadataOccurrence::Only,
        Some(RawMetadataOccurrence::Named(name)) if name == "first" => MetadataOccurrence::First,
        Some(RawMetadataOccurrence::Named(name)) if name == "last" => MetadataOccurrence::Last,
        Some(RawMetadataOccurrence::Named(name)) if name == "all" => MetadataOccurrence::All,
        Some(RawMetadataOccurrence::Index { index }) => MetadataOccurrence::Index(index),
        Some(RawMetadataOccurrence::Named(name)) => {
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "projector '{projector}' input '{input}' uses unsupported metadata occurrence '{name}'"
                ),
                span,
            ));
            MetadataOccurrence::Only
        }
    }
}

fn lower_output_target(
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    output: &str,
    raw: &str,
    span: SourceSpan,
) -> ProjectionOutputTarget {
    match raw {
        "candidate" => ProjectionOutputTarget::Candidate,
        "event" => ProjectionOutputTarget::Event,
        "prompt" => ProjectionOutputTarget::Prompt,
        _ => {
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "projector '{projector}' output '{output}' uses unsupported target '{raw}'"
                ),
                span,
            ));
            ProjectionOutputTarget::Candidate
        }
    }
}

fn lower_literal_for_type(
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

fn selector_metadata_target(selector: &SchemaProjectionSelector) -> Option<MetadataTarget> {
    match selector {
        SchemaProjectionSelector::MetadataKey { target, .. }
        | SchemaProjectionSelector::MetadataSet { target, .. } => Some(*target),
        _ => None,
    }
}

fn metadata_target_name(target: MetadataTarget) -> &'static str {
    match target {
        MetadataTarget::Block => "block",
        MetadataTarget::Choice => "choice",
        MetadataTarget::Line => "line",
        MetadataTarget::Project => "project",
    }
}

fn occurrence_name(occurrence: &MetadataOccurrence) -> &'static str {
    match occurrence {
        MetadataOccurrence::Only => "only",
        MetadataOccurrence::First => "first",
        MetadataOccurrence::Last => "last",
        MetadataOccurrence::Index(_) => "index",
        MetadataOccurrence::All => "all",
    }
}

fn ref_name(input_ref: &ProjectionInputRef) -> String {
    match input_ref {
        ProjectionInputRef::Input { name } => format!("input '{name}'"),
        ProjectionInputRef::QueryResult { name } => format!("query result '{name}'"),
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
