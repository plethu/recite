//! Presentation only; the shared project owner retains the manifest baseline and review.
use super::{Writer, text};
use recite_writer_session::project::settings::Session;
use std::path::PathBuf;

#[derive(Default)]
pub struct Panel {
    pub preview: Vec<String>,
    pub page: usize,
    pub preview_open: bool,
    pub import: String,
    pub artifact: Option<PathBuf>,
    pub feedback: Vec<String>,
    pub feedback_page: usize,
}
impl Panel {
    pub fn clear_feedback(&mut self) {
        self.feedback.clear();
        self.feedback_page = 0;
    }
    pub fn toggle_preview(&mut self) {
        self.preview_open = !self.preview_open;
    }
    pub fn replace_preview(&mut self, preview: Vec<String>) {
        self.preview = preview;
        self.page = 0;
    }
    pub fn record_artifact(&mut self, artifact: PathBuf) {
        self.import = artifact.display().to_string();
        self.artifact = Some(artifact);
    }
    pub fn import_path(&self) -> &str {
        self.import.trim()
    }
    pub fn set_import_path(&mut self, path: String) {
        self.import = path;
    }
    pub fn artifact(&self) -> Option<&std::path::Path> {
        self.artifact.as_deref()
    }
}
impl Writer {
    pub fn project_settings(&self) -> Option<&Session> {
        self.session.project()?.files().settings.as_ref()
    }
    pub fn settings_feedback(&mut self, message: String) {
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
    pub fn cancel_project_settings(&mut self) {
        if let Some(session) = self.project_settings() {
            session.cancel();
            self.status =
                "Cancelling project settings action; waiting for its terminal result.".into();
        }
    }
}
