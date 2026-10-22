//! A field save names its exact catalogue entry rather than borrowing source-save policy.
use super::*;
use std::fmt;
use xilem::masonry::{
    core::{Property, UsesProperty},
    widgets::TextInput,
};

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct TranslationTarget(EntryKey);
impl fmt::Debug for TranslationTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TranslationTarget").finish_non_exhaustive()
    }
}
#[derive(Clone, Default, PartialEq)]
pub(crate) struct TranslationField(pub Option<TranslationTarget>);
impl TranslationField {
    pub(super) fn new(key: EntryKey) -> Self {
        Self(Some(TranslationTarget(key)))
    }
}
impl Property for TranslationField {
    fn static_default() -> &'static Self {
        static EMPTY: TranslationField = TranslationField(None);
        &EMPTY
    }
}
impl UsesProperty<TranslationField> for TextInput {}
impl Writer {
    pub(crate) fn save_focused_translation(&mut self, target: &TranslationTarget) {
        self.save_translation_key(&target.0);
    }
}

#[cfg(test)]
mod tests;
