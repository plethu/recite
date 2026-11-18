//! Deciding a noticed change: compare it with the open work, or settle the open document's change
//! in place with Keep mine or Use disk version.
use super::*;

impl Writer {
    /// Compare the selected notice's change with the open work, on its own page.
    pub fn compare_noticed_change(&mut self) {
        if self.phase != Phase::Writing || self.project_action_blocked() {
            return;
        }
        match self.notices.selected() {
            Some((Target::Source(path), Evidence::Source(observation)))
                if self
                    .session
                    .project()
                    .is_some_and(|project| observation.is_current(project.files())) =>
            {
                let path = path.clone();
                if self
                    .session
                    .project()
                    .is_some_and(|project| project.current().path() == path)
                {
                    self.compare_disk_changes();
                } else if let Err(error) =
                    self.begin_navigation(crate::navigation::Target::CompareFile(path))
                {
                    self.status = error;
                }
            }
            Some((_, Evidence::Catalogue(observation)))
                if observation.is_current(&self.localisation) =>
            {
                let scope = observation.scope().clone();
                self.compare_noticed_catalogue(&scope);
            }
            _ => (),
        }
    }
    /// Whether the selected notice is a current change to the open document, which its banner
    /// can settle in place with Keep mine or Use disk version.
    pub fn noticed_change_settles(&self) -> bool {
        matches!(
            self.notices.selected(),
            Some((Target::Source(path), Evidence::Source(observation)))
                if self.session.project().is_some_and(|project| {
                    project.current().path() == path && observation.is_current(project.files())
                })
        )
    }
    /// Keep mine or Use disk version for the open document's noticed change.
    pub fn settle_noticed_change(&mut self, decision: ExternalDecision) {
        if self.phase == Phase::Writing && self.noticed_change_settles() {
            self.settle_disk_changes(decision);
        }
    }
}
