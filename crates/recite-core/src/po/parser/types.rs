use std::ops::Range;

use super::super::{PoDocument, PoEntryField, PoTranslation};
use crate::Diagnostic;

mod diagnostic_kind;

pub use diagnostic_kind::PoDiagnosticKind;

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PoCommentKind {
    Translator,
    Extracted,
    Reference,
    Flag,
    Previous,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoComment {
    pub(super) kind: PoCommentKind,
    pub(super) text: String,
    pub(super) obsolete: bool,
}

impl PoComment {
    #[must_use]
    pub fn kind(&self) -> &PoCommentKind {
        &self.kind
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub const fn is_obsolete(&self) -> bool {
        self.obsolete
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PoPreviousField {
    Context,
    SourceText,
    PluralSourceText,
    Translation,
    PluralTranslation(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoPreviousValue {
    pub(super) field: PoPreviousField,
    pub(super) value: String,
}

impl PoPreviousValue {
    #[must_use]
    pub fn field(&self) -> &PoPreviousField {
        &self.field
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoUnknownField {
    pub(super) keyword: String,
    pub(super) value: String,
    pub(super) obsolete: bool,
}

impl PoUnknownField {
    #[must_use]
    pub fn keyword(&self) -> &str {
        &self.keyword
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    #[must_use]
    pub const fn is_obsolete(&self) -> bool {
        self.obsolete
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoHeader {
    pub(super) key: String,
    pub(super) value: String,
}

impl PoHeader {
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoParseError {
    pub(super) diagnostic: Box<Diagnostic>,
    pub(super) kind: PoDiagnosticKind,
    pub(super) line: usize,
    pub(super) column: usize,
}

impl PoParseError {
    #[must_use]
    pub fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostic
    }

    #[must_use]
    pub fn kind(&self) -> &PoDiagnosticKind {
        &self.kind
    }

    #[must_use]
    pub const fn line(&self) -> usize {
        self.line
    }

    #[must_use]
    pub const fn column(&self) -> usize {
        self.column
    }
}

impl std::fmt::Display for PoParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.diagnostic.message)
    }
}

impl std::error::Error for PoParseError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PoParseReport {
    pub document: Option<PoDocument>,
    pub diagnostics: Vec<Diagnostic>,
}

impl PoParseReport {
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.document.is_some() && self.diagnostics.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PoFieldTarget {
    Context,
    SourceText,
    PluralSourceText,
    Translation,
    PluralTranslation(usize),
    Previous(PoPreviousField),
    Unknown,
}

impl PoFieldTarget {
    pub(crate) fn from_public(field: &PoEntryField) -> Self {
        match field {
            PoEntryField::Context | PoEntryField::Variant => Self::Context,
            PoEntryField::SourceText => Self::SourceText,
            PoEntryField::PluralSourceText => Self::PluralSourceText,
            PoEntryField::Translation => Self::Translation,
            PoEntryField::PluralTranslation(index) => Self::PluralTranslation(*index),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PoFieldRange {
    pub(crate) target: PoFieldTarget,
    pub(crate) range: Range<usize>,
    pub(crate) value_range: Range<usize>,
    pub(crate) keyword: String,
    pub(crate) multiline: bool,
    pub(crate) obsolete: bool,
}

#[derive(Clone, Debug)]
pub(super) struct SourceLine {
    pub(super) number: usize,
    pub(super) start: usize,
    pub(super) content_end: usize,
    pub(super) end: usize,
    pub(super) text: String,
}

#[derive(Default)]
pub(super) struct EntryBuilder {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) context: Option<String>,
    pub(super) source_text: Option<String>,
    pub(super) plural_source_text: Option<String>,
    pub(super) translation: Option<PoTranslation>,
    pub(super) plural_translations: Vec<PoTranslation>,
    pub(super) comments: Vec<PoComment>,
    pub(super) source_id_comments: Vec<(String, Range<usize>)>,
    pub(super) flags: Vec<String>,
    pub(super) previous: Vec<PoPreviousValue>,
    pub(super) unknown_fields: Vec<PoUnknownField>,
    pub(super) obsolete: bool,
    pub(super) fields: Vec<PoFieldRange>,
}

pub(super) struct ActiveField {
    pub(super) target: PoFieldTarget,
    pub(super) keyword: String,
    pub(super) value: String,
    pub(super) start: usize,
    pub(super) value_start: usize,
    pub(super) end: usize,
    pub(super) multiline: bool,
    pub(super) obsolete: bool,
}
