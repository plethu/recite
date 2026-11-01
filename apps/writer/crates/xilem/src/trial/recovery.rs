//! Preview recovery names the manuscript field and reuses checked Apply publication.
use super::*;
use crate::{attention::EditingTarget, navigation::Target};

impl Writer {
    fn stale_trial_edit(&self) -> bool {
        matches!(
            self.trial.preparation,
            Preparation::Idle(Outcome::UnfinishedEdit(_))
        ) && self.unfinished_trial_edit().is_none()
    }
    pub(crate) fn trial_feedback(&self) -> Option<&'static str> {
        if self.stale_trial_edit() {
            None
        } else {
            self.trial
                .feedback(self.session.model().preview_page().is_some())
        }
    }
    pub(crate) fn trial_status(&self) -> Option<&str> {
        (!self.stale_trial_edit() && !self.trial.status.is_empty())
            .then_some(self.trial.status.as_str())
    }
    pub(crate) fn unfinished_trial_edit(&self) -> Option<&EditingTarget> {
        let Preparation::Idle(Outcome::UnfinishedEdit(target)) = &self.trial.preparation else {
            return None;
        };
        (self.session.model().has_draft() && Some(target) == EditingTarget::capture(self).as_ref())
            .then_some(target)
    }
    pub(crate) fn apply_trial_edit(&mut self) {
        if let Some(target) = self.unfinished_trial_edit().cloned() {
            self.navigate(Target::PreviewEdit(target));
        }
    }
    pub(crate) fn return_to_trial_edit(&mut self) {
        if self.unfinished_trial_edit().is_some() {
            if self.map.workspace == Workspace::Map {
                self.map.workspace = Workspace::Script;
            }
            self.return_from_trial();
            self.reveal_navigation();
            self.request_editing();
        }
    }
    pub(crate) fn cancel_trial_preparation(&mut self) {
        if self.navigation.applying_preview() {
            self.cancel_navigation();
        } else {
            self.trial.cancel();
        }
    }
}
