use toml_edit::DocumentMut;

use super::{
    edit::apply_edit,
    export::export_json,
    fingerprint::{source_fingerprint, source_producer_fingerprint},
    lower::lower_source,
    spans,
};
use crate::schema::schema_diagnostic;
use crate::{
    ContentFingerprint, Diagnostic, DiagnosticArgumentValue, ProjectSchema, SchemaFingerprint,
    canonical_schema_fingerprint,
};

/// Result of loading a source-owning TOML schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaSourceLoadReport {
    pub source: Option<SchemaSource>,
    pub diagnostics: Vec<Diagnostic>,
}

/// A parsed, source-owning schema document.
#[derive(Clone, Debug)]
pub struct SchemaSource {
    pub(super) file: String,
    pub(super) document: DocumentMut,
    pub(super) source_text: String,
    schema: ProjectSchema,
    source_fingerprint: ContentFingerprint,
}

impl PartialEq for SchemaSource {
    fn eq(&self, other: &Self) -> bool {
        self.file == other.file
            && self.source_text == other.source_text
            && self.schema == other.schema
    }
}

impl Eq for SchemaSource {}

impl SchemaSource {
    /// Parse a source-owned TOML schema.
    #[must_use]
    pub fn load_str(file: impl Into<String>, source: &str) -> SchemaSourceLoadReport {
        load_schema_source_str(file, source)
    }

    /// The source path used for diagnostics.
    #[must_use]
    pub fn file(&self) -> &str {
        &self.file
    }

    /// Return the current TOML text, including comments and formatting.
    ///
    /// Immediately after loading, this is the exact input text (including
    /// CRLF and a missing final newline). Typed edits retain that newline
    /// policy and untouched CST trivia while re-rendering the edited item.
    #[must_use]
    pub fn source_text(&self) -> String {
        self.source_text.clone()
    }

    /// Borrow the canonical schema lowered from this source.
    #[must_use]
    pub fn schema(&self) -> &ProjectSchema {
        &self.schema
    }

    /// The source-owned semantic fingerprint, including the producer identity.
    #[must_use]
    pub fn source_fingerprint(&self) -> &ContentFingerprint {
        &self.source_fingerprint
    }

    /// The canonical schema fingerprint, excluding producer diagnostics.
    #[must_use]
    pub fn schema_fingerprint(&self) -> SchemaFingerprint {
        canonical_schema_fingerprint(&self.schema)
    }

    /// Emit deterministic generated JSON. This output is read-only and is
    /// accepted by the existing generated-manifest loader.
    #[must_use]
    pub fn export_json(&self) -> String {
        export_json(&self.schema)
    }

    /// Apply one typed, source-preserving edit and revalidate the result.
    /// Invalid edits leave this document unchanged.
    pub fn apply_edit(&mut self, edit: SchemaSourceEdit) -> Result<(), SchemaSourceEditError> {
        apply_edit(self, edit)
    }
}

/// The named declaration maps supported by a source edit.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum SchemaDeclarationKind {
    Type,
    Registry,
    Speaker,
    Condition,
    AvailabilityReason,
    Effect,
    MetadataDomain,
    Metadata,
    ProjectionQuery,
    PresentationProjector,
    Markup,
}

/// Typed operations over the source-owning TOML document.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SchemaSourceEdit {
    SetProducerId(String),
    SetEnumValues {
        name: String,
        values: Vec<String>,
    },
    SetSpeakerDisplayName {
        name: String,
        display_name: Option<String>,
    },
    RemoveDeclaration {
        kind: SchemaDeclarationKind,
        name: String,
    },
}

/// A typed edit failure. No toml_edit implementation types cross this API.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SchemaSourceEditError {
    InvalidArgument(String),
    Diagnostics(Vec<Diagnostic>),
}

impl std::fmt::Display for SchemaSourceEditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidArgument(message) => formatter.write_str(message),
            Self::Diagnostics(diagnostics) => {
                write!(formatter, "{} schema diagnostics", diagnostics.len())
            }
        }
    }
}

impl std::error::Error for SchemaSourceEditError {}

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
/// structured edit.  toml_edit intentionally renders with its own defaults;
/// preserving this small policy keeps untouched author text stable while the
/// edited CST is reparsed for validation.
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
