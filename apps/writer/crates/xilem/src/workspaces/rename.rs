//! Rename composes an explicit proposal, bounded review and exclusive project actions.
mod view;
use super::{Writer, text, tools::ProjectAction};
pub(super) use view::panel;
pub(crate) use view::recovery;
pub(super) struct Change {
    pub caption: String,
    pub before: Option<String>,
    pub after: Option<String>,
}
#[derive(Default)]
pub(super) struct Panel {
    block: String,
    name: String,
    changes: Vec<Change>,
    page: usize,
    report: Vec<String>,
    report_page: usize,
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
        self.report = report.map(text::pages).unwrap_or_default();
        self.report_page = 0;
    }
}
impl Writer {
    pub(crate) fn has_project_history(&self, redo: bool) -> bool {
        self.session.project().is_some_and(|project| {
            project
                .files()
                .has_rename_history(self.session.model(), redo)
        })
    }
    pub(crate) fn undo_redo(&mut self, redo: bool) {
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
    pub(crate) fn review_project_rename(&mut self) {
        let panel = &self.workspaces.rename;
        self.project_action(ProjectAction::ReviewRename {
            block: panel.block.trim().into(),
            name: panel.name.trim().into(),
        });
    }
    pub(crate) fn change_rename_block(&mut self, value: String) {
        if !self.project_action_blocked() {
            self.discard_rename_review();
            self.workspaces.rename.block = value;
        }
    }
    pub(crate) fn change_rename_name(&mut self, value: String) {
        if !self.project_action_blocked() {
            self.discard_rename_review();
            self.workspaces.rename.name = value;
        }
    }
    pub(crate) fn apply_project_rename(&mut self) {
        self.project_action(ProjectAction::ApplyRename);
    }
    pub(crate) fn undo_project_rename(&mut self) {
        self.project_action(ProjectAction::History { redo: false });
    }
    pub(crate) fn redo_project_rename(&mut self) {
        self.project_action(ProjectAction::History { redo: true });
    }
    pub(crate) fn finish_project_recovery(&mut self) {
        self.project_action(ProjectAction::FinishRecovery);
    }
    pub(crate) fn discard_rename_review(&mut self) {
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
