use crate::{Diagnostic, DiagnosticArgumentValue, SourceSpan, schema::schema_diagnostic};

use super::super::diagnostics::MALFORMED_SHAPE;

pub(super) fn finite_f64_literal(
    diagnostics: &mut Vec<Diagnostic>,
    owner: &str,
    number: String,
    span: SourceSpan,
) -> Option<String> {
    if number.parse::<f64>().is_ok_and(f64::is_finite) {
        return Some(number);
    }

    diagnostics.push(schema_diagnostic(
        MALFORMED_SHAPE,
        "diagnostic-schema-001-float-not-representable",
        format!("{owner} must be finite and representable as f64"),
        span,
        [("owner", DiagnosticArgumentValue::String(owner.to_owned()))],
    ));
    None
}
