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
    /// Filter the queue as the text is typed: the query runs now, or, while the catalogue is
    /// checked out by an earlier query or edit, once it returns.
    pub fn set_catalogue_query(&mut self, text: String) {
        self.localisation.query.text = text;
        self.remember_catalogue_location();
        if self.localisation.slot.identity().is_none() {
            return;
        }
        if self.catalogue_busy() {
            self.localisation.query_pending = true;
        } else {
            self.localisation.query_pending = false;
            self.search_catalogue();
        }
    }
    /// Run a query whose text changed while the catalogue was busy, once it is free.
    pub(crate) fn resume_catalogue_query(&mut self) {
        if self.localisation.query_pending && !self.catalogue_busy() {
            self.localisation.query_pending = false;
            self.search_catalogue();
        }
    }
    fn catalogue_busy(&self) -> bool {
        self.localisation.working()
            || self.workspaces.tools.busy()
            || self.work.busy()
            || self.navigation.busy()
            || self.project_jobs_busy()
    }
    pub fn return_to_translation_queue(&mut self) {
        self.remember_catalogue_location();
        self.localisation.destination = Destination::Queue;
        self.remember_catalogue_location();
    }
}

#[cfg(test)]
mod tests;
