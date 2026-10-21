//! Staged values are independent of the immutable running trial.
use super::Writer;
use recite_writer_session::{
    choices::PickerOption, localisation::setup::languages, preview::Inputs,
};
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct Settings {
    pub open: bool,
    pub locale: String,
    pub variant: String,
    pub include_drafts: bool,
    pub values: BTreeMap<String, String>,
    pub binding_page: usize,
    pub query: String,
    pub languages: Vec<PickerOption>,
    pub language_page: usize,
}

impl Settings {
    pub fn capture(&self) -> Inputs {
        Inputs {
            locale: self.locale.clone(),
            variant: self.variant.clone(),
            include_drafts: self.include_drafts,
            values: Some(self.values.clone()),
        }
    }
}

impl Writer {
    pub(crate) fn search_preview_languages(&mut self) {
        let settings = &mut self.trial.settings;
        settings.languages = languages::matches(&settings.query);
        settings.language_page = 0;
    }
}
