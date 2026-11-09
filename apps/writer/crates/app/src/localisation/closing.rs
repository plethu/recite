//! Explicit translation discard still waits for recovery before the source close path.
use super::*;

impl Writer {
    pub fn discard_translations_and_close(&mut self) {
        if self.phase != Phase::ConfirmClose
            || self.work.busy()
            || self.navigation.busy()
            || self.project_jobs_busy()
            || self.localisation.slot.read_only()
            || self.localisation.saving()
        {
            return;
        }
        self.phase = Phase::Closing;
        self.catalogue_action(work::Action::DiscardForClose);
    }
}

#[cfg(test)]
mod tests;
