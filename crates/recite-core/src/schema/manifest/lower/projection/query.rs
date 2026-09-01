use std::collections::{BTreeMap, BTreeSet};

mod args;

use self::args::lower_and_validate_query_args;
use super::super::super::diagnostics::{
    DUPLICATE_DEFINITION, INVALID_TYPE_REFERENCE, MALFORMED_SHAPE,
};
use super::super::super::raw::{
    Named, RawProjectionQueryDefinition, RawProjectionQueryFunctionDefinition,
};
use super::super::super::spans::ManifestSpans;
use super::super::super::validate::{
    PendingTypeReference, duplicate_definition, validate_manifest_name,
};
use super::super::LoweringContext;
use super::super::functions::lower_params_at;
use super::QueryArgumentContext;
use super::reference::lower_input_refs;
use crate::schema::schema_diagnostic;
use crate::schema::{
    ProjectSchema, ProjectionInput, ProjectionQueryDefinition, ProjectionQueryFunctionDefinition,
};
use crate::{Diagnostic, DiagnosticArgumentValue};

macro_rules! text_args {
    ($($name:literal => $value:expr),* $(,)?) => {
        [$(($name, DiagnosticArgumentValue::String($value.into()))),*]
    };
}

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
        let entry_path = vec!["projection_queries".to_owned(), entry.name.clone()];
        let name_span = spans.key_span_at(file, source, &entry_path, &entry.name);
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

        let params = lower_params_at(
            &mut LoweringContext::new(file, source, spans, diagnostics),
            &format!("projection query function '{}'", entry.name),
            &entry.value.params,
            pending_type_refs,
            &entry_path,
        );
        let mut returns_path = entry_path.clone();
        returns_path.push("returns".to_owned());
        let (returns, returns_span, returns_valid) =
            super::super::types::lower_type_reference_at_with_context(
                &mut LoweringContext::new(file, source, spans, diagnostics),
                &entry.value.returns,
                &returns_path,
                format!(
                    "projection query function '{}' has invalid return type '{}'",
                    entry.name, entry.value.returns
                ),
                super::super::types::TypeReferenceContext::QueryReturn {
                    function: entry.name.clone(),
                },
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
            diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-query-max-calls",
                format!(
                    "projection query function '{}' max_calls_per_event must be greater than zero",
                    entry.name
                ),
                name_span,
                [(
                    "function",
                    DiagnosticArgumentValue::String(entry.name.clone()),
                )],
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

pub(super) fn lower_projector_queries(
    lowering: &mut LoweringContext<'_>,
    schema: &ProjectSchema,
    projector: &str,
    inputs: &[ProjectionInput],
    raw_queries: Vec<Named<RawProjectionQueryDefinition>>,
    projector_path: &[String],
) -> BTreeMap<String, ProjectionQueryDefinition> {
    let input_types = inputs
        .iter()
        .map(|input| (input.name.as_str(), &input.type_ref))
        .collect::<BTreeMap<_, _>>();
    let mut query_types = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut lowered = BTreeMap::new();

    for raw_query in raw_queries {
        let mut query_path = projector_path.to_vec();
        query_path.extend(["queries".to_owned(), raw_query.name.clone()]);
        let query_span = lowering.key_span_at(&query_path, &raw_query.name);
        validate_manifest_name(
            lowering.diagnostics,
            "projection query name",
            &raw_query.name,
            query_span.clone(),
        );
        if !seen.insert(raw_query.name.clone()) {
            lowering.diagnostics.push(schema_diagnostic(
                DUPLICATE_DEFINITION,
                "diagnostic-schema-003-projection-query",
                format!("projector '{projector}' repeats query '{}'", raw_query.name),
                query_span,
                text_args!("projector" => projector, "query" => raw_query.name.clone()),
            ));
            continue;
        }
        let mut function_path = query_path.clone();
        function_path.push("function".to_owned());
        let function_span = lowering.value_span_at(&function_path, &raw_query.value.function);
        let function = schema.projection_queries.get(&raw_query.value.function);
        let Some(function) = function else {
            lowering.diagnostics.push(schema_diagnostic(
                INVALID_TYPE_REFERENCE,
                "diagnostic-schema-004-unknown-query-function",
                format!(
                    "projector '{projector}' query '{}' references unknown projection query function '{}'",
                    raw_query.name, raw_query.value.function
                ),
                function_span,
                text_args!("projector" => projector, "query" => raw_query.name.clone(), "function" => raw_query.value.function.clone()),
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
            lowering.diagnostics,
            QueryArgumentContext {
                projector,
                query: &raw_query.name,
                function: &raw_query.value.function,
                params: &function.params,
                input_types: &input_types,
                query_types: &query_types,
            },
            raw_query.value.args,
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
