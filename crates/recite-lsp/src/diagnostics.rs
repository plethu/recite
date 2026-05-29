use lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString,
    PublishDiagnosticsParams, Uri,
};
use recite_core::{Diagnostic as ReciteDiagnostic, DiagnosticSeverity as ReciteSeverity};

use crate::position::span_to_range;

pub(crate) fn publish_diagnostics(
    uri: Uri,
    text: &str,
    version: Option<i32>,
    diagnostics: &[ReciteDiagnostic],
) -> PublishDiagnosticsParams {
    let mut diagnostics = diagnostics
        .iter()
        .map(|diagnostic| to_lsp_diagnostic(&uri, text, diagnostic))
        .collect::<Vec<_>>();
    diagnostics.sort_by(diagnostic_sort_key);

    PublishDiagnosticsParams::new(uri, diagnostics, version)
}

pub(crate) fn clear_diagnostics(uri: Uri) -> PublishDiagnosticsParams {
    PublishDiagnosticsParams::new(uri, Vec::new(), None)
}

fn to_lsp_diagnostic(uri: &Uri, text: &str, diagnostic: &ReciteDiagnostic) -> Diagnostic {
    Diagnostic {
        range: span_to_range(text, &diagnostic.span),
        severity: Some(to_lsp_severity(diagnostic.severity)),
        code: Some(NumberOrString::String(diagnostic.code.as_str().to_owned())),
        code_description: None,
        source: Some("recite".to_owned()),
        message: diagnostic.message.clone(),
        related_information: related_information(uri, text, diagnostic),
        tags: None,
        data: None,
    }
}

fn related_information(
    fallback_uri: &Uri,
    text: &str,
    diagnostic: &ReciteDiagnostic,
) -> Option<Vec<DiagnosticRelatedInformation>> {
    if diagnostic.related.is_empty() {
        return None;
    }

    Some(
        diagnostic
            .related
            .iter()
            .map(|related| DiagnosticRelatedInformation {
                location: Location {
                    uri: related
                        .span
                        .file
                        .parse::<Uri>()
                        .unwrap_or_else(|_| fallback_uri.clone()),
                    range: span_to_range(text, &related.span),
                },
                message: related.message.clone(),
            })
            .collect(),
    )
}

fn to_lsp_severity(severity: ReciteSeverity) -> DiagnosticSeverity {
    match severity {
        ReciteSeverity::Error => DiagnosticSeverity::ERROR,
        ReciteSeverity::Warning => DiagnosticSeverity::WARNING,
        ReciteSeverity::Information => DiagnosticSeverity::INFORMATION,
        ReciteSeverity::Hint => DiagnosticSeverity::HINT,
    }
}

fn diagnostic_sort_key(left: &Diagnostic, right: &Diagnostic) -> std::cmp::Ordering {
    (
        left.range.start.line,
        left.range.start.character,
        left.range.end.line,
        left.range.end.character,
        code_sort_key(left),
        &left.message,
    )
        .cmp(&(
            right.range.start.line,
            right.range.start.character,
            right.range.end.line,
            right.range.end.character,
            code_sort_key(right),
            &right.message,
        ))
}

fn code_sort_key(diagnostic: &Diagnostic) -> &str {
    match diagnostic.code.as_ref() {
        Some(NumberOrString::String(code)) => code,
        Some(NumberOrString::Number(_)) | None => "",
    }
}
