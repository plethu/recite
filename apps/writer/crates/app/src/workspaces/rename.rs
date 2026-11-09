//! Rename composes an explicit proposal, bounded review and exclusive project actions.
pub mod selection;
#[cfg(test)]
mod tests;
pub use super::{Writer, text, tools::ProjectAction};
pub(super) use recite_writer_session::comparison::ComparisonRow as Change;
pub(crate) use selection::BeatChoice;
#[derive(Default)]
pub struct Panel {
    pub block: String,
    pub name: String,
    pub changes: Vec<Change>,
    pub page: usize,
    pub report: Option<Report>,
    pub report_page: usize,
}
pub enum Report {
    Information(Vec<String>),
    Problem(Vec<String>),
}
impl Report {
    pub fn pages(&self) -> &[String] {
        match self {
            Self::Information(pages) | Self::Problem(pages) => pages,
        }
    }
}
impl Panel {
    pub(super) fn choose_block(&mut self, block: Option<String>) {
        if self.block.is_empty() {
            self.block = block.unwrap_or_default();
        }
    }
    pub(super) fn replace(&mut self, pages: Vec<Change>) {
        self.changes = pages;
        self.page = 0;
    }
    pub(super) fn report(&mut self, report: Option<&str>) {
        self.report = report.map(|report| Report::Problem(text::pages(report)));
        self.report_page = 0;
    }
    pub(super) fn inform(&mut self, report: &str) {
        self.report = Some(Report::Information(text::pages(report)));
        self.report_page = 0;
    }
}
impl Writer {
    pub(crate) fn retire_published_rename_notice(&mut self) {
        if matches!(self.workspaces.rename.report, Some(Report::Information(_)))
            && self
                .session
                .project()
                .is_some_and(|project| !project.files().project_edit_pending(self.session.model()))
        {
            self.workspaces.rename.report(None);
        }
    }
    pub fn has_project_history(&self, redo: bool) -> bool {
        self.session.project().is_some_and(|project| {
            project
                .files()
                .has_rename_history(self.session.model(), redo)
        })
    }
    pub fn undo_redo(&mut self, redo: bool) {
        if self.has_project_history(redo) {
            self.project_action(ProjectAction::History { redo });
        } else {
            self.navigate(crate::navigation::Target::Edit(if redo {
                recite_writer_model::EditOperation::Redo
            } else {
                recite_writer_model::EditOperation::Undo
            }));
        }
    }
    pub fn review_project_rename(&mut self) {
        let panel = &self.workspaces.rename;
        self.project_action(ProjectAction::ReviewRename {
            block: panel.block.trim().into(),
            name: panel.name.trim().into(),
        });
    }
    pub fn change_rename_block(&mut self, value: String) {
        if !self.project_action_blocked() {
            self.discard_rename_review();
            self.workspaces.rename.block = value;
        }
    }
    pub fn change_rename_name(&mut self, value: String) {
        if !self.project_action_blocked() {
            self.discard_rename_review();
            self.workspaces.rename.name = value;
        }
    }
    pub fn apply_project_rename(&mut self) {
        self.project_action(ProjectAction::ApplyRename);
    }
    pub fn undo_project_rename(&mut self) {
        self.project_action(ProjectAction::History { redo: false });
    }
    pub fn redo_project_rename(&mut self) {
        self.project_action(ProjectAction::History { redo: true });
    }
    pub fn finish_project_recovery(&mut self) {
        self.project_action(ProjectAction::FinishRecovery);
    }
    pub fn discard_rename_review(&mut self) {
        if self.project_action_blocked() {
            return;
        }
        if let Some(project) = self.session.project_mut() {
            project.parts_mut().0.rename_review = None;
        }
        self.workspaces.rename.replace(vec![]);
        self.workspaces.rename.report(None);
        self.status = "Rename review discarded. Project files remain unchanged.".into();
    }
}
