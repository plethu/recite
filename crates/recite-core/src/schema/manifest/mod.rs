mod diagnostics;
mod lower;
mod raw;
mod spans;
mod validate;

use crate::Diagnostic;

use super::ProjectSchema;
use diagnostics::{MALFORMED_SHAPE, diagnostic};
use lower::lower_manifest;
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
    let file = file.into();
    let raw = match serde_json::from_str::<RawManifest>(source) {
        Ok(raw) => raw,
        Err(error) => {
            return SchemaLoadReport {
                schema: None,
                diagnostics: vec![diagnostic(
                    MALFORMED_SHAPE,
                    format!("malformed schema manifest: {error}"),
                    json_error_span(&file, &error),
                )],
            };
        }
    };

    lower_manifest(file, source, raw)
}
