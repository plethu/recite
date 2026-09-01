pub use super::types::{
    SchemaDeclarationKind, SchemaSource, SchemaSourceEdit, SchemaSourceEditError,
    SchemaSourceLoadReport, SchemaSourceStaleDetails,
};
use super::{
    fingerprint::{source_fingerprint, source_producer_fingerprint},
    lower::lower_source,
    spans,
};
use crate::DiagnosticArgumentValue;
use crate::schema::schema_diagnostic;

/// Load the standalone producer's versioned TOML source.
#[must_use]
pub fn load_schema_source_str(file: impl Into<String>, source: &str) -> SchemaSourceLoadReport {
    let file = file.into();
    let parsed = match toml_edit::Document::parse(source.to_owned()) {
        Ok(document) => document,
        Err(error) => {
            return SchemaSourceLoadReport {
                source: None,
                diagnostics: vec![schema_diagnostic(
                    super::diagnostics::MALFORMED_SHAPE,
                    "diagnostic-schema-001-toml-parse",
                    format!("malformed schema source: {}", error.message()),
                    spans::error_span(&file, source, error.span()),
                    [(
                        "detail",
                        DiagnosticArgumentValue::String(error.message().to_owned()),
                    )],
                )],
            };
        }
    };

    let toml_spans = crate::schema::manifest::TomlSpanIndex::from_document(&parsed);
    let document = parsed.into_mut();
    let (schema, diagnostics) = lower_source(&file, source, &document, &toml_spans);
    let Some(schema) = schema else {
        return SchemaSourceLoadReport {
            source: None,
            diagnostics,
        };
    };
    let source_fingerprint = source_fingerprint(&schema);
    let mut schema = schema;
    if let Some(fingerprint) = source_producer_fingerprint(&schema, &source_fingerprint)
        && let Some(metadata) = schema.producer_metadata.as_mut()
    {
        metadata.producer_fingerprints.push(fingerprint);
        metadata.producer_fingerprints.sort();
    }
    SchemaSourceLoadReport {
        source: Some(SchemaSource {
            file,
            document,
            source_text: source.to_owned(),
            schema,
            source_fingerprint,
        }),
        diagnostics,
    }
}

/// Match the source document's newline and final-newline policy after a
/// structured edit while retaining untouched CST trivia.
pub(super) fn apply_source_layout_policy(rendered: String, original: &str) -> String {
    let uses_crlf = original.contains("\r\n");
    let has_final_newline = original.ends_with('\n');
    let mut rendered = rendered.replace("\r\n", "\n");
    if !has_final_newline && rendered.ends_with('\n') {
        rendered.pop();
    }
    if uses_crlf {
        rendered = rendered.replace('\n', "\r\n");
    }
    rendered
}
