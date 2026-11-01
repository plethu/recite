//! Catalogue destinations participate in the existing checked navigation history.
use super::*;

impl Writer {
    pub(super) fn remember_catalogue_location(&mut self) {
        if self.phase == Phase::Writing
            && self.workspaces.screen == Screen::Localisation
            && !self.navigation.busy()
        {
            self.remember_location();
        }
    }
    pub(super) fn set_catalogue_query(&mut self, text: String) {
        self.localisation.query.text = text;
        self.remember_catalogue_location();
    }
    pub(super) fn return_to_translation_queue(&mut self) {
        self.remember_catalogue_location();
        self.localisation.destination = Destination::Queue;
        self.remember_catalogue_location();
    }
}

#[cfg(test)]
mod tests;
