//! A comparison is an observation of one exact session; decisions never stage the field implicitly.
#[cfg(test)]
mod tests;
mod view;
use super::{
    Screen, Writer, text,
    tools::{ExternalAction, ProjectAction},
};
use crate::external_editor::Request;
use recite_writer_session::{
    comparison::ComparisonRow,
    project::{ExternalComparison, ExternalDecision, SourceRepairArtifact},
};
use std::{path::PathBuf, sync::Arc};
pub(super) use view::panel;

pub(super) struct Inspection {
    pub path: PathBuf,
    pub changes: Vec<ComparisonRow>,
    pub pending: Option<Vec<String>>,
    pub artifact: Option<SourceRepairArtifact>,
    pub archive: Option<PathBuf>,
}
impl Inspection {
    pub(super) fn prepare(comparison: &ExternalComparison) -> Self {
        Self {
            path: comparison.path().to_owned(),
            changes: ComparisonRow::source(
                "",
                comparison.writer_source(),
                comparison.disk_source(),
            )
            .into_iter()
            .filter(|row| row.before != row.after)
            .flat_map(text::change_pages)
            .collect(),
            pending: comparison
                .pending_draft()
                .map(|draft| text::pages(draft.draft_snapshot().as_str())),
            artifact: comparison.artifact().cloned(),
            archive: comparison.archive().map(PathBuf::from),
        }
    }
}
#[derive(Default)]
pub(super) struct Panel {
    inspection: Option<Inspection>,
    page: usize,
    draft_page: usize,
    report: Vec<String>,
    report_page: usize,
}
impl Panel {
    pub(super) fn exchange(&mut self, inspection: &mut Option<Inspection>) {
        std::mem::swap(&mut self.inspection, inspection);
        self.page = 0;
        self.draft_page = 0;
    }
    pub(super) fn replace(&mut self, inspection: Option<Inspection>) {
        self.inspection = inspection;
        self.page = 0;
        self.draft_page = 0;
    }
    pub(super) fn report(&mut self, report: Option<&str>) {
        self.report = report.map(text::pages).unwrap_or_default();
        self.report_page = 0;
    }
}
impl Writer {
    pub(crate) fn external_resolution_pending(&self) -> bool {
        self.session
            .project()
            .is_some_and(|project| project.files().external.is_some())
    }
    pub(crate) fn compare_disk_changes(&mut self) {
        if self.project_action_blocked() || self.session.project().is_none() {
            return;
        }
        self.attention.leave_focus();
        self.cancel_map_gesture();
        self.workspaces.screen = Screen::External;
        self.external_action(ExternalAction::Compare);
    }
    pub(crate) fn keep_writer_version(&mut self) {
        self.external_action(ExternalAction::Resolve(ExternalDecision::KeepWriter));
    }
    pub(crate) fn use_inspected_disk(&mut self) {
        self.external_action(ExternalAction::Resolve(ExternalDecision::UseInspectedDisk));
    }
    pub(crate) fn export_merge_draft(&mut self) {
        self.external_action(ExternalAction::ExportMerge);
    }
    pub(crate) fn import_reviewed_merge(&mut self) {
        if let Some(artifact) = self
            .workspaces
            .external
            .inspection
            .as_ref()
            .and_then(|inspection| inspection.artifact.as_ref())
        {
            self.external_action(ExternalAction::ImportMerge(artifact.source.clone()));
        }
    }
    fn external_action(&mut self, action: ExternalAction) {
        self.project_action(ProjectAction::External(action));
    }
    pub(crate) fn open_merge_draft(&mut self) {
        if self.project_action_blocked() || self.external.busy() {
            return;
        }
        let Some(project) = self.session.project() else {
            return;
        };
        let Some(artifact) = self
            .workspaces
            .external
            .inspection
            .as_ref()
            .and_then(|inspection| inspection.artifact.as_ref())
        else {
            return;
        };
        self.launch_editor(Arc::new(Request::merge_draft(
            project,
            artifact.source.clone(),
            self.editor.clone(),
        )));
    }
    fn external_comparison_current(&self) -> bool {
        self.session.project().is_some_and(|project| {
            project.files().external.as_ref().is_some_and(|comparison| {
                comparison.is_current(project.files(), self.session.model())
            })
        })
    }
}
