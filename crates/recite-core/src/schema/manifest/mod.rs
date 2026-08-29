mod diagnostics;
pub(crate) mod lower;
mod producer;
pub(crate) mod raw;
mod raw_json;
pub(crate) mod raw_toml;
mod raw_value;
mod spans;
pub(crate) mod validate;

pub(crate) use spans::TomlSpanIndex;

use crate::{Diagnostic, DiagnosticArgumentValue};
use serde_json::Value;

use super::ProjectSchema;
use super::schema_diagnostic;
use diagnostics::MALFORMED_SHAPE;
use lower::{ManifestLoadOptions, lower_manifest};
use raw::RawManifest;
use spans::json_error_span;

/// Result of loading a generated schema manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaLoadReport {
    pub schema: Option<ProjectSchema>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Load a generated JSON schema manifest into the canonical schema model.
#[must_use]
pub fn load_schema_manifest_str(file: impl Into<String>, source: &str) -> SchemaLoadReport {
    load_schema_manifest_str_with_options(file, source, ManifestLoadOptions::default())
}

/// Load a manifest for producer freshness comparison.
///
/// Freshness comparison needs to report duplicate producer fingerprints as a
/// typed comparison result. This mode retains those entries in the lowered
/// model, while ordinary schema loading remains strict and diagnostic.
#[must_use]
pub fn load_schema_manifest_for_freshness_str(
    file: impl Into<String>,
    source: &str,
) -> SchemaLoadReport {
    load_schema_manifest_str_with_options(
        file,
        source,
        ManifestLoadOptions {
            allow_duplicate_producer_fingerprints: true,
        },
    )
}

fn load_schema_manifest_str_with_options(
    file: impl Into<String>,
    source: &str,
    options: ManifestLoadOptions,
) -> SchemaLoadReport {
    let file = file.into();
    let mut raw = match serde_json::from_str::<RawManifest>(source) {
        Ok(raw) => raw,
        Err(error) => {
            return SchemaLoadReport {
                schema: None,
                diagnostics: vec![schema_diagnostic(
                    MALFORMED_SHAPE,
                    "diagnostic-schema-001-json-parse",
                    format!("malformed schema manifest: {error}"),
                    json_error_span(&file, &error),
                    [("detail", DiagnosticArgumentValue::String(error.to_string()))],
                )],
            };
        }
    };
    if let Ok(json) = serde_json::from_str::<Value>(source) {
        raw_json::preserve_json_number_lexemes(&mut raw, &json);
    }

    lower_manifest(file, source, raw, options)
}
