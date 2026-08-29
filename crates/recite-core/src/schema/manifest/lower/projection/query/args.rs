use std::collections::BTreeMap;

use super::super::super::super::diagnostics::MALFORMED_SHAPE;
use super::super::super::super::raw::RawProjectionInputRef;
use super::super::reference::{lower_input_ref, validate_ref_type};
use crate::schema::schema_diagnostic;
use crate::schema::{ParameterDefinition, ProjectionInputRef, SchemaTypeRef};
use crate::{Diagnostic, DiagnosticArgumentValue, SourceSpan};

#[expect(
    clippy::too_many_arguments,
    reason = "projection query argument validation carries query, type, and span context"
)]
pub(super) fn lower_and_validate_query_args(
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
        diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-query-arg-count",
            format!(
                "projector '{projector}' query '{query}' passes {} args to projection query function '{function}', expected {}",
                raw_args.len(),
                params.len()
            ),
            span.clone(),
            [
                ("projector", DiagnosticArgumentValue::String(projector.to_owned())),
                ("query", DiagnosticArgumentValue::String(query.to_owned())),
                ("actual", DiagnosticArgumentValue::Integer(raw_args.len() as i64)),
                ("function", DiagnosticArgumentValue::String(function.to_owned())),
                ("expected", DiagnosticArgumentValue::Integer(params.len() as i64)),
            ],
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
