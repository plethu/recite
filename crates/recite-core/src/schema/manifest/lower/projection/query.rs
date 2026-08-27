use std::collections::{BTreeMap, BTreeSet};

use super::super::super::diagnostics::{
    DUPLICATE_DEFINITION, INVALID_TYPE_REFERENCE, MALFORMED_SHAPE,
};
use super::super::super::raw::{
    Named, RawProjectionInputRef, RawProjectionQueryDefinition,
    RawProjectionQueryFunctionDefinition,
};
use super::super::super::spans::ManifestSpans;
use super::super::super::validate::{
    PendingTypeReference, duplicate_definition, validate_manifest_name,
};
use super::super::functions::lower_params;
use super::super::types::lower_type_reference;
use super::reference::{lower_input_ref, lower_input_refs, validate_ref_type};
use crate::schema::{
    ParameterDefinition, ProjectSchema, ProjectionInput, ProjectionInputRef,
    ProjectionQueryDefinition, ProjectionQueryFunctionDefinition, SchemaTypeRef,
};
use crate::{Diagnostic, SourceSpan};

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

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
pub(super) fn lower_projector_queries(
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
    reason = "manifest lowering helpers carry shared JSON span, schema, and type context"
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
