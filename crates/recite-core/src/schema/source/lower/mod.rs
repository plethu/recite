use super::{diagnostics::MALFORMED_SHAPE, raw};
use crate::schema::manifest::{
    TomlSpanIndex,
    lower::{ManifestLoadOptions, ManifestSourceFormat, lower_manifest_with_format},
    raw::{Named, RawManifest, RawValue},
};
use crate::{Diagnostic, DiagnosticArgumentValue, ProjectSchema, schema::schema_diagnostic};
use toml_edit::{DocumentMut, Item};
#[rustfmt::skip]
macro_rules! source_diagnostic {
    ($id:literal, $message:expr, $span:expr $(,)?) => { schema_diagnostic(MALFORMED_SHAPE, concat!("diagnostic-schema-001-", $id), $message, $span, std::iter::empty::<(&str, DiagnosticArgumentValue)>()) };
    ($id:literal, $message:expr, $span:expr, $arguments:expr $(,)?) => { schema_diagnostic(MALFORMED_SHAPE, concat!("diagnostic-schema-001-", $id), $message, $span, $arguments) };
}
/// Parse source TOML into the canonical raw representation and invoke the
/// existing manifest lowerer. No source-specific semantic validator lives in
/// this module: parity with generated JSON is therefore a property of the
/// canonical lowering path.
pub(super) fn lower_source(
    file: &str,
    source: &str,
    document: &DocumentMut,
    toml_spans: &TomlSpanIndex,
) -> (Option<ProjectSchema>, Vec<Diagnostic>) {
    let mut diagnostics = validate_source_shape(file, source, document, toml_spans);
    diagnostics.extend(reject_non_finite_numbers(
        file, source, document, toml_spans,
    ));
    if !diagnostics.is_empty() {
        return (None, diagnostics);
    }
    let canonical = raw::canonical_document(document);
    let mut raw = match toml_edit::de::from_document::<RawManifest>(canonical) {
        Ok(raw) => raw,
        Err(error) => {
            diagnostics.push(source_diagnostic!(
                "toml-decode",
                format!("malformed schema source: {}", error.message()),
                super::spans::error_span(file, source, error.span()),
                [(
                    "detail",
                    DiagnosticArgumentValue::String(error.message().to_owned()),
                )],
            ));
            return (None, diagnostics);
        }
    };
    super::super::manifest::raw_toml::preserve_toml_float_lexemes(&mut raw, source, toml_spans);
    diagnostics.extend(reject_legacy_reason_bindings(
        file, source, toml_spans, &raw,
    ));
    if !diagnostics.is_empty() {
        return (None, diagnostics);
    }
    let report = lower_manifest_with_format(
        file.to_owned(),
        source,
        raw,
        ManifestLoadOptions::default(),
        ManifestSourceFormat::Toml,
        Some(toml_spans),
    );
    (report.schema, report.diagnostics)
}
fn reject_non_finite_numbers(
    file: &str,
    source: &str,
    document: &DocumentMut,
    spans: &TomlSpanIndex,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for (key, item) in document.as_table().iter() {
        reject_non_finite_item(
            file,
            source,
            item,
            &[key.to_owned()],
            spans,
            &mut diagnostics,
        );
    }
    diagnostics
}
fn reject_non_finite_item(
    file: &str,
    source: &str,
    item: &Item,
    path: &[String],
    spans: &TomlSpanIndex,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(value) = item.as_value() {
        reject_non_finite_value(file, source, value, path, spans, item.span(), diagnostics);
    }
    if let Some(table) = item.as_table() {
        for (key, child) in table.iter() {
            let mut child_path = path.to_vec();
            child_path.push(key.to_owned());
            reject_non_finite_item(file, source, child, &child_path, spans, diagnostics);
        }
    }
    if let Some(tables) = item.as_array_of_tables() {
        for (index, table) in tables.iter().enumerate() {
            let mut table_path = path.to_vec();
            table_path.push(format!("[{index}]"));
            for (key, child) in table.iter() {
                let mut child_path = table_path.clone();
                child_path.push(key.to_owned());
                reject_non_finite_item(file, source, child, &child_path, spans, diagnostics);
            }
        }
    }
}
fn reject_non_finite_value(
    file: &str,
    source: &str,
    value: &toml_edit::Value,
    path: &[String],
    spans: &TomlSpanIndex,
    item_range: Option<std::ops::Range<usize>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(number) = value.as_float()
        && !number.is_finite()
    {
        diagnostics.push(source_diagnostic!(
            "source-non-finite",
            "non-finite TOML numbers are not supported",
            super::spans::range_span(
                file,
                source,
                spans
                    .value_range(path)
                    .or_else(|| item_range.or_else(|| value.span())),
            ),
        ));
    }
    if let Some(array) = value.as_array() {
        for (index, element) in array.iter().enumerate() {
            let mut element_path = path.to_vec();
            element_path.push(format!("[{index}]"));
            reject_non_finite_value(
                file,
                source,
                element,
                &element_path,
                spans,
                element.span(),
                diagnostics,
            );
        }
    }
    if let Some(table) = value.as_inline_table() {
        for (key, child) in table.iter() {
            let mut child_path = path.to_vec();
            child_path.push(key.to_owned());
            reject_non_finite_value(
                file,
                source,
                child,
                &child_path,
                spans,
                child.span(),
                diagnostics,
            );
        }
    }
}
fn reject_legacy_reason_bindings(
    file: &str,
    source: &str,
    spans: &TomlSpanIndex,
    raw: &RawManifest,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for condition in &raw.conditions {
        let Some(mapping) = condition.value.availability_reason.as_ref() else {
            continue;
        };
        for Named { name, value } in &mapping.args {
            let legacy = match value {
                RawValue::String(value) => value.starts_with('$'),
                RawValue::Object(fields) => {
                    fields.get("kind").is_none()
                        && fields
                            .get("value")
                            .and_then(RawValue::as_str)
                            .is_some_and(|value| value.starts_with('$'))
                }
                _ => false,
            };
            if legacy {
                diagnostics.push(source_diagnostic!(
                    "source-legacy-binding",
                    "TOML availability reason bindings must use { kind = \"binding\", name = \"...\" }",
                    arg_span(file, source, spans, &condition.name, name, true)
                ));
                continue;
            }
            let Some(fields) = (match value {
                RawValue::Object(fields) => Some(fields),
                _ => None,
            }) else {
                continue;
            };
            let Some(kind) = fields.get("kind").and_then(RawValue::as_str) else {
                continue;
            };
            let allowed = match kind {
                "binding" => ["kind", "name"].as_slice(),
                "literal" => ["kind", "value"].as_slice(),
                _ => continue,
            };
            if fields
                .keys()
                .any(|field| !allowed.contains(&field.as_str()))
            {
                diagnostics.push(source_diagnostic!(
                    "source-tagged-field",
                    format!(
                        "availability reason argument '{name}' contains an unknown tagged field"
                    ),
                    arg_span(file, source, spans, &condition.name, name, false),
                    [("name", DiagnosticArgumentValue::String(name.clone()))],
                ));
            }
        }
    }
    diagnostics
}
fn arg_span(
    file: &str,
    source: &str,
    spans: &TomlSpanIndex,
    condition: &str,
    name: &str,
    value: bool,
) -> crate::SourceSpan {
    let path = vec![
        "conditions".to_owned(),
        condition.to_owned(),
        "availability_reason".to_owned(),
        "args".to_owned(),
        name.to_owned(),
    ];
    let range = if value {
        spans.value_range(&path).or_else(|| {
            let mut value_path = path.clone();
            value_path.push("value".to_owned());
            spans.value_range(&value_path)
        })
    } else {
        spans.key_range(&path)
    };
    super::spans::error_span(file, source, range)
}
fn validate_source_shape(
    file: &str,
    source: &str,
    document: &DocumentMut,
    spans: &TomlSpanIndex,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for key in raw::GENERATED_ONLY_FIELDS {
        if document.contains_key(key) {
            diagnostics.push(source_diagnostic!(
                "source-generated-field",
                format!("generated-only field '{key}' is not accepted in authoritative TOML"),
                super::spans::key_span(file, source, spans, &[], key, false),
                [("key", DiagnosticArgumentValue::String((*key).to_owned()))],
            ));
        }
    }
    let Some(Item::Table(producer)) = document.get("producer") else {
        diagnostics.push(source_diagnostic!(
            "source-producer-required",
            "a [producer] table with a stable id is required",
            super::spans::document_span(file),
        ));
        return diagnostics;
    };
    let Some(id) = producer.get("id").and_then(Item::as_str) else {
        diagnostics.push(source_diagnostic!(
            "source-producer-id-required",
            "producer id is required",
            super::spans::table_span(file, source, spans, &["producer".to_owned()]),
        ));
        return diagnostics;
    };
    if id.trim().is_empty() {
        diagnostics.push(source_diagnostic!(
            "source-producer-id-empty",
            "producer id must not be empty",
            super::spans::key_span(file, source, spans, &["producer".to_owned()], "id", true),
        ));
    }
    if let Some(kind) = producer.get("kind").and_then(Item::as_str)
        && kind != "standalone"
    {
        diagnostics.push(source_diagnostic!(
            "source-producer-kind",
            "standalone TOML producer kind must be 'standalone'",
            super::spans::key_span(file, source, spans, &["producer".to_owned()], "kind", true),
        ));
    }
    diagnostics
}
