//! Closing a window: a close or exit prompt while work is unsaved or confirmation is asked, a
//! checkpoint for recovery, and the way back to writing.
use super::*;

impl Writer {
    pub fn request_close(&mut self) {
        if let Err(error) = self.require_finished_composition() {
            self.status = error;
            return;
        }
        if self.work.busy() || self.navigation.busy() || self.project_jobs_busy() {
            self.status = "An editor action is still finishing. Your work remains open.".into();
            return;
        }
        if self.session.dirty() || self.localisation.dirty() {
            self.phase = Phase::ConfirmClose;
        } else if self.config.writer.confirm_exit {
            self.phase = Phase::ConfirmExit;
        } else {
            self.close_with_recovery();
        }
    }
    pub fn close_with_recovery(&mut self) {
        if self.work.busy() || self.navigation.busy() || self.project_jobs_busy() {
            self.status = "Wait for the current editor action before closing.".into();
            return;
        }
        if self.localisation.has_catalogue() {
            self.phase = Phase::Closing;
            self.flush_catalogue_before_close();
            return;
        }
        self.close_sources_with_recovery();
    }
    pub(crate) fn close_sources_with_recovery(&mut self) {
        let Some(project) = self.session.project_mut() else {
            self.phase = Phase::Closed;
            return;
        };
        match project.checkpoint() {
            Ok(()) => self.phase = Phase::Closed,
            Err(error) => {
                self.status = feedback::file(&error);
                self.phase = Phase::ConfirmClose;
            }
        }
    }
    /// Leave a close or exit prompt and continue working.
    pub fn keep_editing(&mut self) {
        self.phase = Phase::Writing;
        self.work.keep_editing();
        self.localisation.keep_editing();
    }
    /// Leave a close prompt to review translations that still have unsaved drafts.
    pub fn open_unsaved_translations(&mut self) {
        self.keep_editing();
        self.enter_localisation();
        self.localisation.query.filter = localisation::Filter::Unsaved;
        self.search_catalogue();
    }
}
