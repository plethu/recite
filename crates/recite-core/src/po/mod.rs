//! Lossless gettext PO documents for dialogue catalogue authoring.
//!
//! This layer owns PO source representation and editing only. Locale lookup is
//! a runtime-provider concern, and Recite-owned UI resources remain Fluent.

use std::ops::Range;

mod document;
mod entry;
mod parser;
mod write;

pub use document::PoEditError;
pub use parser::{PluralRuleError, evaluate_plural_form, validate_plural_rule};
pub use parser::{
    PoComment, PoCommentKind, PoDiagnosticKind, PoHeader, PoParseError, PoParseReport,
    PoPreviousField, PoPreviousValue, PoUnknownField,
};
pub use write::{PoIoError, PoWriteError};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PoEntryId(usize);
impl PoEntryId {
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self(index)
    }
    #[must_use]
    pub const fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoTranslation {
    pub(super) index: Option<usize>,
    pub(super) text: String,
}
impl PoTranslation {
    #[must_use]
    pub const fn index(&self) -> Option<usize> {
        self.index
    }
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PoEntryField {
    Context,
    SourceText,
    PluralSourceText,
    Translation,
    PluralTranslation(usize),
    Variant,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoEdit {
    pub entry: PoEntryId,
    pub field: PoEntryField,
    pub value: String,
}
impl PoEdit {
    #[must_use]
    pub fn new(entry: PoEntryId, field: PoEntryField, value: impl Into<String>) -> Self {
        Self {
            entry,
            field,
            value: value.into(),
        }
    }
    #[must_use]
    pub fn translation(entry: PoEntryId, value: impl Into<String>) -> Self {
        Self::new(entry, PoEntryField::Translation, value)
    }
    #[must_use]
    pub fn plural_translation(entry: PoEntryId, arm: usize, value: impl Into<String>) -> Self {
        Self::new(entry, PoEntryField::PluralTranslation(arm), value)
    }
    #[must_use]
    pub fn variant(entry: PoEntryId, value: impl Into<String>) -> Self {
        Self::new(entry, PoEntryField::Variant, value)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PoDocumentFingerprint([u8; 32]);
impl PoDocumentFingerprint {
    #[must_use]
    pub fn from_source(source: &str) -> Self {
        Self::from_bytes(source.as_bytes())
    }
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    #[must_use]
    pub const fn digest(&self) -> &[u8; 32] {
        &self.0
    }
}
impl std::fmt::Display for PoDocumentFingerprint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoDocument {
    pub(super) source_name: String,
    pub(super) source: String,
    pub(super) entries: Vec<PoEntry>,
    pub(super) headers: Vec<PoHeader>,
    pub(super) line_ending: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoEntry {
    pub(super) id: PoEntryId,
    pub(super) range: Range<usize>,
    pub(super) start_line: usize,
    pub(super) context: Option<String>,
    pub(super) source_text: String,
    pub(super) plural_source_text: Option<String>,
    pub(super) translation: Option<PoTranslation>,
    pub(super) plural_translations: Vec<PoTranslation>,
    pub(super) comments: Vec<PoComment>,
    pub(super) flags: Vec<String>,
    pub(super) previous: Vec<PoPreviousValue>,
    pub(super) unknown_fields: Vec<PoUnknownField>,
    pub(super) obsolete: bool,
    pub(super) header: bool,
    pub(super) fields: Vec<parser::PoFieldRange>,
}
