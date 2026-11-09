//! Native translation field identity over `recite_writer_app::localisation::keyboard`.
use super::*;
use xilem::masonry::{
    core::{Property, UsesProperty},
    widgets::TextInput,
};

#[derive(Clone, Default, PartialEq)]
pub struct TranslationField(pub Option<TranslationTarget>);
impl TranslationField {
    pub(crate) fn new(key: recite_writer_app::localisation::EntryKey) -> Self {
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
