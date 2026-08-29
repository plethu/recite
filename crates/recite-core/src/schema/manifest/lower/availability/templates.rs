use std::collections::BTreeSet;

use super::super::super::diagnostics::MALFORMED_SHAPE;
use crate::schema::{ParameterDefinition, schema_diagnostic};
use crate::{
    Diagnostic, DiagnosticArgumentValue, PlaceholderSyntaxKind, SourceSpan,
    extract_placeholder_names,
};

pub(super) fn validate_template_placeholders(
    diagnostics: &mut Vec<Diagnostic>,
    reason: &str,
    template: &str,
    params: &[ParameterDefinition],
    span: SourceSpan,
) {
    let placeholders = match extract_placeholder_names(template) {
        Ok(placeholders) => placeholders,
        Err(error) => {
            let (presentation_id, arguments) = match error.kind() {
                PlaceholderSyntaxKind::Unterminated => (
                    "diagnostic-schema-001-availability-template-unterminated",
                    vec![("reason", DiagnosticArgumentValue::String(reason.to_owned()))],
                ),
                PlaceholderSyntaxKind::InvalidName(name) => (
                    "diagnostic-schema-001-availability-template-invalid-name",
                    vec![
                        ("reason", DiagnosticArgumentValue::String(reason.to_owned())),
                        ("name", DiagnosticArgumentValue::String(name.clone())),
                    ],
                ),
                PlaceholderSyntaxKind::UnescapedClosingBrace => (
                    "diagnostic-schema-001-availability-template-unescaped-closing-brace",
                    vec![("reason", DiagnosticArgumentValue::String(reason.to_owned()))],
                ),
            };
            diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                presentation_id,
                format!(
                    "availability reason '{reason}' template has invalid placeholder syntax: {}",
                    error.message()
                ),
                span,
                arguments,
            ));
            return;
        }
    };
    let param_names = params
        .iter()
        .map(|param| param.name.as_str())
        .collect::<BTreeSet<_>>();

    for placeholder in &placeholders {
        if !param_names.contains(placeholder.as_str()) {
            diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-availability-template-unknown-param",
                format!(
                    "availability reason '{reason}' template references unknown parameter '{placeholder}'"
                ),
                span.clone(),
                [
                    ("reason", DiagnosticArgumentValue::String(reason.to_owned())),
                    (
                        "placeholder",
                        DiagnosticArgumentValue::String(placeholder.clone()),
                    ),
                ],
            ));
        }
    }
    for param in params {
        if !placeholders.contains(&param.name) {
            diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-availability-template-unused-param",
                format!(
                    "availability reason '{reason}' parameter '{}' is not used in its template",
                    param.name
                ),
                span.clone(),
                [
                    ("reason", DiagnosticArgumentValue::String(reason.to_owned())),
                    (
                        "parameter",
                        DiagnosticArgumentValue::String(param.name.clone()),
                    ),
                ],
            ));
        }
    }
}
