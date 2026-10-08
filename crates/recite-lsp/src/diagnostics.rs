use lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString,
    PublishDiagnosticsParams, Uri,
};
use recite_core::{
    Diagnostic as ReciteDiagnostic, DiagnosticArgumentValue, DiagnosticRecord,
    DiagnosticRecordError, DiagnosticSeverity as ReciteSeverity, SourceSpan,
};
use recite_ui::{CatalogError, RenderedRelatedDiagnostic, UiCatalog};
use thiserror::Error;

use crate::position::DocumentLines;
use std::cell::OnceCell;
#[cfg(test)]
mod tests;

/// Borrowed source view from the workspace for resolving diagnostic spans.
/// Compiler spans use project-relative paths, while LSP clients need both a
/// target URI and that target document's text for correct UTF-16 conversion.
#[derive(Debug)]
pub(crate) struct DiagnosticSource<'a> {
    pub(crate) path: String,
    pub(crate) uri: &'a Uri,
    pub(crate) text: &'a str,
}

#[derive(Debug, Error)]
pub(crate) enum DiagnosticPublishError {
    #[error("failed to record diagnostic `{code}`: {source}")]
    Record {
        code: String,
        #[source]
        source: DiagnosticRecordError,
    },
    #[error("failed to render diagnostic `{code}`: {source}")]
    Render {
        code: String,
        #[source]
        source: CatalogError,
    },
}

pub(crate) fn publish_diagnostics(
    uri: Uri,
    text: &str,
    version: Option<i32>,
    diagnostics: &[ReciteDiagnostic],
    catalog: &UiCatalog,
    sources: &[DiagnosticSource<'_>],
) -> Result<PublishDiagnosticsParams, DiagnosticPublishError> {
    if diagnostics.is_empty() {
        return Ok(PublishDiagnosticsParams::new(uri, Vec::new(), version));
    }
    let lines = DocumentLines::new(text);
    let sources = sources
        .iter()
        .map(|source| IndexedSource {
            source,
            lines: OnceCell::new(),
        })
        .collect::<Vec<_>>();
    let mut records = diagnostics
        .iter()
        .enumerate()
        .map(|(index, diagnostic)| {
            diagnostic
                .record()
                .map(|record| IndexedDiagnostic {
                    index,
                    sort_key: diagnostic_sort_key(&lines, &record),
                    record,
                })
                .map_err(|source| DiagnosticPublishError::Record {
                    code: diagnostic.code.as_str().to_owned(),
                    source,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    records.sort_by(|left, right| {
        left.sort_key
            .cmp(&right.sort_key)
            .then_with(|| left.index.cmp(&right.index))
    });

    let diagnostics = records
        .into_iter()
        .map(|item| to_lsp_diagnostic(&lines, &item.record, catalog, &sources))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PublishDiagnosticsParams::new(uri, diagnostics, version))
}

struct IndexedDiagnostic {
    index: usize,
    record: DiagnosticRecord,
    sort_key: DiagnosticSortKey,
}

struct IndexedSource<'a> {
    source: &'a DiagnosticSource<'a>,
    lines: OnceCell<DocumentLines<'a>>,
}

pub(crate) fn clear_diagnostics(uri: Uri, version: Option<i32>) -> PublishDiagnosticsParams {
    PublishDiagnosticsParams::new(uri, Vec::new(), version)
}

fn to_lsp_diagnostic(
    lines: &DocumentLines<'_>,
    record: &DiagnosticRecord,
    catalog: &UiCatalog,
    sources: &[IndexedSource<'_>],
) -> Result<Diagnostic, DiagnosticPublishError> {
    let rendered =
        catalog
            .render_diagnostic(record)
            .map_err(|source| DiagnosticPublishError::Render {
                code: record.code.as_str().to_owned(),
                source,
            })?;

    Ok(Diagnostic {
        range: lines.span_to_range(&record.span),
        severity: Some(to_lsp_severity(record.severity)),
        code: Some(NumberOrString::String(record.code.as_str().to_owned())),
        code_description: None,
        source: Some("recite".to_owned()),
        message: rendered.primary_text,
        related_information: related_information(&rendered.related, sources),
        tags: None,
        data: None,
    })
}

fn related_information(
    related: &[RenderedRelatedDiagnostic],
    sources: &[IndexedSource<'_>],
) -> Option<Vec<DiagnosticRelatedInformation>> {
    let related = related
        .iter()
        .filter_map(|related| {
            let (uri, lines) = resolve_source(sources, &related.span.file)?;
            Some(DiagnosticRelatedInformation {
                location: Location {
                    uri: uri.clone(),
                    range: lines.span_to_range(&related.span),
                },
                message: related.text.clone(),
            })
        })
        .collect::<Vec<_>>();
    (!related.is_empty()).then_some(related)
}

fn resolve_source<'a>(
    sources: &'a [IndexedSource<'_>],
    path: &str,
) -> Option<(&'a Uri, &'a DocumentLines<'a>)> {
    sources
        .iter()
        .find(|source| source.source.path == path || source.source.uri.as_str() == path)
        .map(|source| {
            (
                source.source.uri,
                source
                    .lines
                    .get_or_init(|| DocumentLines::new(source.source.text)),
            )
        })
}

fn to_lsp_severity(severity: ReciteSeverity) -> DiagnosticSeverity {
    match severity {
        ReciteSeverity::Error => DiagnosticSeverity::ERROR,
        ReciteSeverity::Warning => DiagnosticSeverity::WARNING,
        ReciteSeverity::Information => DiagnosticSeverity::INFORMATION,
        ReciteSeverity::Hint => DiagnosticSeverity::HINT,
    }
}

fn diagnostic_sort_key(lines: &DocumentLines<'_>, record: &DiagnosticRecord) -> DiagnosticSortKey {
    let range = lines.span_to_range(&record.span);
    DiagnosticSortKey {
        primary_range: (
            range.start.line,
            range.start.character,
            range.end.line,
            range.end.character,
        ),
        source_span: source_span_sort_key(&record.span),
        code: record.code.as_str().to_owned(),
        presentation_id: record.presentation.id().as_str().to_owned(),
        arguments: record
            .presentation
            .arguments()
            .iter()
            .map(|(name, value)| (name.clone(), argument_sort_key(value)))
            .collect(),
    }
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct DiagnosticSortKey {
    primary_range: (u32, u32, u32, u32),
    source_span: (String, u32, u32, Option<(u32, u32)>),
    code: String,
    presentation_id: String,
    arguments: Vec<(String, DiagnosticArgumentSortKey)>,
}

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
enum DiagnosticArgumentSortKey {
    String(String),
    Integer(i64),
    Float(u64),
    Boolean(bool),
    Future(String),
}

fn source_span_sort_key(span: &SourceSpan) -> (String, u32, u32, Option<(u32, u32)>) {
    (
        span.file.clone(),
        span.start.line(),
        span.start.column(),
        span.end.map(|end| (end.line(), end.column())),
    )
}

fn argument_sort_key(value: &DiagnosticArgumentValue) -> DiagnosticArgumentSortKey {
    match value {
        DiagnosticArgumentValue::String(value) => DiagnosticArgumentSortKey::String(value.clone()),
        DiagnosticArgumentValue::Integer(value) => DiagnosticArgumentSortKey::Integer(*value),
        DiagnosticArgumentValue::Float(value) => {
            DiagnosticArgumentSortKey::Float(value.as_f64().to_bits())
        }
        DiagnosticArgumentValue::Boolean(value) => DiagnosticArgumentSortKey::Boolean(*value),
        _ => DiagnosticArgumentSortKey::Future(
            serde_json::to_string(value)
                .unwrap_or_else(|error| format!("argument serialization failed: {error}")),
        ),
    }
}
