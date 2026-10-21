use super::*;
use recite_writer_session::{
    choices::PickerOption,
    localisation::{create, setup::languages},
};

#[derive(Default)]
pub(super) struct Creation {
    pub query: String,
    pub choices: Vec<PickerOption>,
    pub page: usize,
    pub selected: String,
}
impl Writer {
    pub(super) fn new_catalogue(&mut self) {
        self.remember_catalogue_location();
        self.localisation.destination = Destination::Create;
        self.remember_catalogue_location();
    }
    pub(super) fn search_catalogue_languages(&mut self) {
        let state = &mut self.localisation.creation;
        state.choices = languages::matches(&state.query);
        state.page = 0;
        state.selected.clear();
    }
    pub(super) fn create_catalogue(&mut self) {
        match create::language(&self.localisation.creation.selected) {
            Ok(locale) => self.source_catalogue_action(work::SourceAction::Create(locale)),
            Err(error) => self.catalogue_feedback(error),
        }
    }
}
