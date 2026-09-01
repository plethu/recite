use super::super::super::super::diagnostics::MALFORMED_SHAPE;
use super::super::super::super::raw::RawProjectionInputRef;
use super::super::QueryArgumentContext;
use super::super::reference::{lower_input_ref, validate_ref_type};
use crate::schema::ProjectionInputRef;
use crate::schema::schema_diagnostic;
use crate::{Diagnostic, DiagnosticArgumentValue, SourceSpan};

pub(super) fn lower_and_validate_query_args(
    diagnostics: &mut Vec<Diagnostic>,
    context: QueryArgumentContext<'_>,
    raw_args: Vec<RawProjectionInputRef>,
    span: SourceSpan,
) -> Vec<ProjectionInputRef> {
    if raw_args.len() != context.params.len() {
        diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-query-arg-count",
            format!(
                "projector '{}' query '{}' passes {} args to projection query function '{}', expected {}",
                context.projector,
                context.query,
                raw_args.len(),
                context.function,
                context.params.len()
            ),
            span.clone(),
            [
                (
                    "projector",
                    DiagnosticArgumentValue::String(context.projector.to_owned()),
                ),
                ("query", DiagnosticArgumentValue::String(context.query.to_owned())),
                ("actual", DiagnosticArgumentValue::Integer(raw_args.len() as i64)),
                (
                    "function",
                    DiagnosticArgumentValue::String(context.function.to_owned()),
                ),
                (
                    "expected",
                    DiagnosticArgumentValue::Integer(context.params.len() as i64),
                ),
            ],
        ));
    }
    raw_args
        .into_iter()
        .enumerate()
        .map(|(index, raw)| {
            let input_ref = lower_input_ref(raw);
            if let Some(param) = context.params.get(index) {
                validate_ref_type(
                    diagnostics,
                    super::super::ReferenceTypeContext {
                        projector: context.projector,
                        owner: &format!("query '{}' argument '{}'", context.query, param.name),
                        expected: &param.type_ref,
                        input_types: context.input_types,
                        query_types: context.query_types,
                    },
                    &input_ref,
                    span.clone(),
                );
            }
            input_ref
        })
        .collect()
}
