use super::*;
use recite_writer_session::builds::job;

impl Writer {
    pub fn build_saved(&mut self) {
        let selected = self
            .session
            .project()
            .and_then(|project| project.files().builds.selected.clone());
        self.build_selected(selected);
    }
    pub(crate) fn build_selected(&mut self, asset: Option<String>) {
        if self.declaration_action_blocked() {
            return;
        }
        let Some(project) = self.session.project_mut() else {
            return;
        };
        let (files, _) = project.parts_mut();
        if files.builds.busy() {
            return;
        }
        match job::Job::start(files.root().to_owned(), asset) {
            Ok(job) => {
                self.workspaces.build_feedback.clear();
                self.workspaces.feedback_page = 0;
                files.builds.job = Some(job);
                files.builds.result = None;
                self.status = "Building saved project inputs… You can keep writing.".into();
            }
            Err(error) => self.status = error,
        }
    }
    pub fn cancel_build(&mut self) {
        if let Some(project) = self.session.project() {
            project.files().builds.cancel();
            self.status = "Cancelling build; waiting for its terminal result.".into();
        }
    }
    pub fn poll_project_jobs(&mut self) {
        self.poll_schema_generation();
        let Some(project) = self.session.project_mut() else {
            return;
        };
        let (files, _) = project.parts_mut();
        if let Some(result) = files.builds.job.as_ref().and_then(job::Job::poll) {
            let message = result.message();
            self.status = message
                .lines()
                .next()
                .unwrap_or("Build finished")
                .chars()
                .take(240)
                .collect();
            self.workspaces.build_feedback = super::text::pages(&message);
            self.workspaces.feedback_page = 0;
            files.builds.job = None;
            files.builds.result = Some(result);
        }
    }
}
