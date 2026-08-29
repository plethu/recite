use std::ops::Range;

use super::{PoEditError, PoEntry, PoEntryField};
use crate::po::parser;

impl PoEntry {
    #[must_use]
    pub const fn id(&self) -> super::PoEntryId {
        self.id
    }
    #[must_use]
    pub fn context(&self) -> Option<&str> {
        self.context.as_deref()
    }
    #[must_use]
    pub fn variant(&self) -> Option<&str> {
        self.context.as_deref().and_then(|context| {
            context
                .split_once('&')
                .map(|(_, variant)| variant)
                .filter(|variant| !variant.is_empty())
        })
    }
    #[must_use]
    pub fn stable_id_metadata(&self) -> Option<&str> {
        self.comments.iter().find_map(|comment| {
            comment
                .text()
                .strip_prefix("source id:")
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
    }
    /// Source line on which this PO entry begins.
    #[must_use]
    pub const fn line(&self) -> usize {
        self.start_line
    }
    #[must_use]
    pub fn source_text(&self) -> &str {
        &self.source_text
    }
    #[must_use]
    pub fn plural_source_text(&self) -> Option<&str> {
        self.plural_source_text.as_deref()
    }
    #[must_use]
    pub fn translation(&self) -> Option<&str> {
        self.translation.as_ref().map(super::PoTranslation::text)
    }
    #[must_use]
    pub fn translations(&self) -> &[super::PoTranslation] {
        &self.plural_translations
    }
    #[must_use]
    pub fn translation_arms(&self) -> &[super::PoTranslation] {
        &self.plural_translations
    }
    #[must_use]
    pub fn plural_translations(&self) -> &[super::PoTranslation] {
        &self.plural_translations
    }
    #[must_use]
    pub fn plural_translation(&self, arm: usize) -> Option<&str> {
        self.plural_translations
            .iter()
            .find(|translation| translation.index() == Some(arm))
            .map(super::PoTranslation::text)
    }
    #[must_use]
    pub fn comments(&self) -> &[parser::PoComment] {
        &self.comments
    }
    #[must_use]
    pub fn flags(&self) -> &[String] {
        &self.flags
    }
    #[must_use]
    pub fn previous(&self) -> &[parser::PoPreviousValue] {
        &self.previous
    }
    #[must_use]
    pub fn unknown_fields(&self) -> &[parser::PoUnknownField] {
        &self.unknown_fields
    }
    #[must_use]
    pub const fn is_obsolete(&self) -> bool {
        self.obsolete
    }
    #[must_use]
    pub const fn is_header(&self) -> bool {
        self.header
    }
    #[must_use]
    pub const fn is_plural(&self) -> bool {
        self.plural_source_text.is_some()
    }
    pub(super) fn edit_range(
        &self,
        field: &PoEntryField,
    ) -> Result<(Range<usize>, String, bool, bool), PoEditError> {
        let target = parser::PoFieldTarget::from_public(field);
        if let Some(item) = self.fields.iter().find(|item| item.target == target) {
            return Ok((
                item.range.clone(),
                item.keyword.clone(),
                item.multiline,
                item.obsolete,
            ));
        }
        if matches!(field, PoEntryField::Context) {
            let start = self
                .fields
                .iter()
                .find(|item| item.target == parser::PoFieldTarget::SourceText)
                .map(|item| item.range.start)
                .ok_or(PoEditError::FieldNotFound(field.clone()))?;
            return Ok((start..start, "msgctxt".to_owned(), false, self.obsolete));
        }
        Err(PoEditError::FieldNotFound(field.clone()))
    }
}
