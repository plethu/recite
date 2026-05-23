use super::super::diagnostics::{INVALID_TYPE_REFERENCE, diagnostic};
use super::super::spans::ManifestSpans;
use super::super::validate::parse_type_ref;
use crate::schema::SchemaTypeRef;
use crate::{Diagnostic, SourceSpan};

pub(super) fn lower_type_reference(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    value: &str,
    invalid_message: String,
) -> (SchemaTypeRef, SourceSpan, bool) {
    let type_ref_span = spans.next_value_span(file, source, value);
    match parse_type_ref(value) {
        Some(type_ref) => (type_ref, type_ref_span, true),
        None => {
            diagnostics.push(diagnostic(
                INVALID_TYPE_REFERENCE,
                invalid_message,
                type_ref_span.clone(),
            ));
            (SchemaTypeRef::String, type_ref_span, false)
        }
    }
}
