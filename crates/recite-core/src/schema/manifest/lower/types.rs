use super::super::diagnostics::INVALID_TYPE_REFERENCE;
use super::super::spans::ManifestSpans;
use super::super::validate::parse_type_ref;
use crate::schema::{SchemaTypeRef, schema_diagnostic};
use crate::{Diagnostic, DiagnosticArgumentValue, SourceSpan};

#[derive(Clone, Debug)]
pub(super) enum TypeReferenceContext {
    Metadata {
        metadata: String,
    },
    Parameter {
        parameter: String,
    },
    ProjectionInput {
        projector: String,
        input: String,
    },
    ProjectionOutput {
        projector: String,
        output: String,
        binding: String,
    },
    QueryReturn {
        function: String,
    },
}

#[expect(
    clippy::too_many_arguments,
    reason = "typed type-reference lowering carries shared source span and semantic context"
)]
pub(super) fn lower_type_reference_at_with_context(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    value: &str,
    path: &[String],
    invalid_message: String,
    context: TypeReferenceContext,
) -> (SchemaTypeRef, SourceSpan, bool) {
    let type_ref_span = spans.value_span_at(file, source, path, value);
    lower_type_reference_with_context(diagnostics, value, type_ref_span, invalid_message, context)
}

fn lower_type_reference_with_context(
    diagnostics: &mut Vec<Diagnostic>,
    value: &str,
    type_ref_span: SourceSpan,
    invalid_message: String,
    context: TypeReferenceContext,
) -> (SchemaTypeRef, SourceSpan, bool) {
    match parse_type_ref(value) {
        Some(type_ref) => (type_ref, type_ref_span, true),
        None => {
            let (presentation_id, arguments) = match context {
                TypeReferenceContext::Metadata { metadata } => (
                    "diagnostic-schema-004-invalid-metadata-type",
                    vec![
                        ("metadata", DiagnosticArgumentValue::String(metadata)),
                        (
                            "type_ref",
                            DiagnosticArgumentValue::String(value.to_owned()),
                        ),
                    ],
                ),
                TypeReferenceContext::Parameter { parameter } => (
                    "diagnostic-schema-004-invalid-parameter-type",
                    vec![
                        ("parameter", DiagnosticArgumentValue::String(parameter)),
                        (
                            "type_ref",
                            DiagnosticArgumentValue::String(value.to_owned()),
                        ),
                    ],
                ),
                TypeReferenceContext::ProjectionInput { projector, input } => (
                    "diagnostic-schema-004-invalid-projection-input-type",
                    vec![
                        ("projector", DiagnosticArgumentValue::String(projector)),
                        ("input", DiagnosticArgumentValue::String(input)),
                        (
                            "type_ref",
                            DiagnosticArgumentValue::String(value.to_owned()),
                        ),
                    ],
                ),
                TypeReferenceContext::ProjectionOutput {
                    projector,
                    output,
                    binding,
                } => (
                    "diagnostic-schema-004-invalid-projection-output-type",
                    vec![
                        ("projector", DiagnosticArgumentValue::String(projector)),
                        ("output", DiagnosticArgumentValue::String(output)),
                        ("binding", DiagnosticArgumentValue::String(binding)),
                        (
                            "type_ref",
                            DiagnosticArgumentValue::String(value.to_owned()),
                        ),
                    ],
                ),
                TypeReferenceContext::QueryReturn { function } => (
                    "diagnostic-schema-004-invalid-query-return-type",
                    vec![
                        ("function", DiagnosticArgumentValue::String(function)),
                        (
                            "type_ref",
                            DiagnosticArgumentValue::String(value.to_owned()),
                        ),
                    ],
                ),
            };
            diagnostics.push(schema_diagnostic(
                INVALID_TYPE_REFERENCE,
                presentation_id,
                invalid_message,
                type_ref_span.clone(),
                arguments,
            ));
            (SchemaTypeRef::String, type_ref_span, false)
        }
    }
}
