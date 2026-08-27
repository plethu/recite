use std::collections::BTreeMap;

use super::super::super::diagnostics::{INVALID_TYPE_REFERENCE, MALFORMED_SHAPE};
use super::super::super::raw::RawProjectionInputRef;
use super::super::super::spans::ManifestSpans;
use super::super::super::validate::PendingTypeReference;
use crate::schema::{ProjectSchema, ProjectionInputRef, ProjectionQueryDefinition, SchemaTypeRef};
use crate::{Diagnostic, SourceSpan};

pub(super) fn lower_input_refs(raw_refs: Vec<RawProjectionInputRef>) -> Vec<ProjectionInputRef> {
    raw_refs.into_iter().map(lower_input_ref).collect()
}

pub(super) fn lower_input_ref(raw: RawProjectionInputRef) -> ProjectionInputRef {
    match raw {
        RawProjectionInputRef::Input { input } => ProjectionInputRef::Input { name: input },
        RawProjectionInputRef::QueryResult { query_result } => {
            ProjectionInputRef::QueryResult { name: query_result }
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
pub(super) fn lower_output_type_ref(
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
    let (type_ref, type_ref_span, valid) = super::super::types::lower_type_reference(
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
    reason = "projection reference validation carries owner, expected type, and source maps"
)]
pub(super) fn validate_ref_type<T>(
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

pub(super) fn query_result_types(
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

fn ref_name(input_ref: &ProjectionInputRef) -> String {
    match input_ref {
        ProjectionInputRef::Input { name } => format!("input '{name}'"),
        ProjectionInputRef::QueryResult { name } => format!("query result '{name}'"),
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
