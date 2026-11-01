//! Presentation only; the shared project owner retains the manifest baseline and review.
mod view;
use super::{Writer, text};
use recite_writer_session::project::settings::Session;
use std::path::PathBuf;
pub(super) use view::panel;

#[derive(Default)]
pub(super) struct Panel {
    preview: Vec<String>,
    page: usize,
    preview_open: bool,
    import: String,
    artifact: Option<PathBuf>,
    feedback: Vec<String>,
    feedback_page: usize,
}
impl Panel {
    pub(super) fn clear_feedback(&mut self) {
        self.feedback.clear();
        self.feedback_page = 0;
    }
    pub(super) fn toggle_preview(&mut self) {
        self.preview_open = !self.preview_open;
    }
    pub(super) fn replace_preview(&mut self, preview: Vec<String>) {
        self.preview = preview;
        self.page = 0;
    }
    pub(super) fn record_artifact(&mut self, artifact: PathBuf) {
        self.import = artifact.display().to_string();
        self.artifact = Some(artifact);
    }
    pub(super) fn import_path(&self) -> &str {
        self.import.trim()
    }
    pub(super) fn set_import_path(&mut self, path: String) {
        self.import = path;
    }
    pub(super) fn artifact(&self) -> Option<&std::path::Path> {
        self.artifact.as_deref()
    }
}
impl Writer {
    pub(super) fn project_settings(&self) -> Option<&Session> {
        self.session.project()?.files().settings.as_ref()
    }
    pub(super) fn settings_feedback(&mut self, message: String) {
        self.status = message
            .lines()
            .next()
            .unwrap_or("Project settings action finished")
            .chars()
            .take(240)
            .collect();
        self.workspaces.settings.feedback = text::pages(&message);
        self.workspaces.settings.feedback_page = 0;
    }
    pub(crate) fn cancel_project_settings(&mut self) {
        if let Some(session) = self.project_settings() {
            session.cancel();
            self.status =
                "Cancelling project settings action; waiting for its terminal result.".into();
        }
    }
}
