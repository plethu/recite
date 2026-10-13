use super::{PoDocument, PoEdit, PoEntry, PoEntryField};
use crate::po::parser;

impl PoDocument {
    pub fn parse(source: impl Into<String>) -> Result<Self, parser::PoParseError> {
        Self::parse_with_path("<po>", source)
    }
    pub fn parse_with_path(
        source_name: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<Self, parser::PoParseError> {
        parser::parse_document(source_name.into(), source.into())
    }
    pub fn from_str_with_path(
        source_name: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<Self, parser::PoParseError> {
        Self::parse_with_path(source_name, source)
    }
    #[must_use]
    pub fn parse_report(source: impl Into<String>) -> parser::PoParseReport {
        Self::parse_report_with_path("<po>", source)
    }
    #[must_use]
    pub fn parse_report_with_path(
        source_name: impl Into<String>,
        source: impl Into<String>,
    ) -> parser::PoParseReport {
        match Self::parse_with_path(source_name, source) {
            Ok(document) => parser::PoParseReport {
                document: Some(document),
                diagnostics: Vec::new(),
            },
            Err(error) => parser::PoParseReport {
                document: None,
                diagnostics: vec![error.diagnostic().clone()],
            },
        }
    }
    #[must_use]
    pub fn parse_with_diagnostics(
        source_name: impl Into<String>,
        source: impl Into<String>,
    ) -> parser::PoParseReport {
        Self::parse_report_with_path(source_name, source)
    }
    #[must_use]
    pub fn source_name(&self) -> &str {
        &self.source_name
    }
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }
    /// Retain this immutable source revision without copying its text.
    ///
    /// Edits install a new validated source; existing snapshots stay unchanged.
    #[must_use]
    pub fn source_snapshot(&self) -> super::Arc<String> {
        self.source.clone()
    }
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.source.as_bytes()
    }
    #[must_use]
    pub fn entries(&self) -> &[PoEntry] {
        &self.entries
    }
    #[must_use]
    pub fn entry(&self, id: super::PoEntryId) -> Option<&PoEntry> {
        self.entries.get(id.index())
    }
    /// Find the unique active entry for a durable catalogue key.
    ///
    /// Parsing rejects duplicate active keys, while fuzzy and obsolete records
    /// remain available through [`Self::entries`] but never win lookup.
    #[must_use]
    pub fn find(&self, context: &str, source_text: &str) -> Option<&PoEntry> {
        self.entries.iter().find(|entry| {
            !entry.is_header()
                && !entry.is_obsolete()
                && !entry.flags().iter().any(|flag| flag == "fuzzy")
                && entry.context.as_deref() == Some(context)
                && entry.source_text == source_text
        })
    }
    #[must_use]
    pub fn headers(&self) -> &[parser::PoHeader] {
        &self.headers
    }
    #[must_use]
    pub fn fingerprint(&self) -> super::PoDocumentFingerprint {
        super::PoDocumentFingerprint::from_source(&self.source)
    }

    pub fn apply_edit(&mut self, edit: PoEdit) -> Result<(), PoEditError> {
        self.apply_edits([edit])
    }

    pub fn edit(&mut self, edit: PoEdit) -> Result<(), PoEditError> {
        self.apply_edit(edit)
    }
    pub fn apply_edits(
        &mut self,
        edits: impl IntoIterator<Item = PoEdit>,
    ) -> Result<(), PoEditError> {
        self.apply_updates(edits.into_iter().map(super::PoUpdate::Field))
    }
}

impl std::str::FromStr for PoDocument {
    type Err = Box<parser::PoParseError>;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        Self::parse(source).map_err(Box::new)
    }
}

impl std::fmt::Display for PoDocument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.source)
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PoEditError {
    #[error("PO entry {0:?} does not exist")]
    EntryNotFound(super::PoEntryId),
    #[error("PO entry does not contain editable field {0:?}")]
    FieldNotFound(PoEntryField),
    #[error("PO entry has no context for a variant edit")]
    MissingContext,
    #[error("invalid PO variant `{0}`")]
    InvalidVariant(String),
    #[error("overlapping PO edits are not a single logical batch")]
    OverlappingEdits,
    #[error("edit would produce an invalid PO document: {0}")]
    InvalidDocument(parser::PoParseError),
}
