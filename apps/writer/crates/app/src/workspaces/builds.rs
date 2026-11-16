use super::*;
use recite_writer_session::builds::job;

/// A finished build's result as its slip shows it.
pub struct BuildResult {
    /// Whether the build published or was cancelled, rather than failing.
    pub succeeded: bool,
    /// What happened: the outcome's first line, or that a failed build did not complete.
    pub headline: String,
    /// A failure's first line under its headline; its full report is on the build page.
    pub detail: Option<String>,
}

/// The headline of every failed build.
const FAILED: &str = "Build did not complete";

/// The longest summary a result slip or the status line sets.
const SUMMARY_CHARS: usize = 240;

impl Writer {
    /// The Build command: build the saved project now, with the outputs chosen on the build page
    /// or all of them. Progress shows in the status bar and the outcome on a result slip.
    pub fn build_scenes(&mut self) {
        self.build_saved();
    }
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
                self.workspaces.build_result = false;
                self.workspaces.build_options = false;
                self.workspaces.build_feedback.clear();
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
            self.status = summary(&message);
            self.workspaces.build_feedback = super::text::pages(&message);
            // On the build page its report shows in place; elsewhere a slip carries it.
            self.workspaces.build_result = self.workspaces.screen != Screen::Build;
            files.builds.job = None;
            files.builds.result = Some(result);
        }
    }
    /// The result slip's content while it is open.
    pub fn build_result(&self) -> Option<BuildResult> {
        let result = self
            .session
            .project()?
            .files()
            .builds
            .result
            .as_ref()
            .filter(|_| self.workspaces.build_result)?;
        let summary = summary(&result.message());
        Some(if result.is_success() {
            BuildResult {
                succeeded: true,
                headline: summary,
                detail: None,
            }
        } else {
            BuildResult {
                succeeded: false,
                headline: FAILED.into(),
                detail: (!summary.starts_with(FAILED)).then_some(summary),
            }
        })
    }
    pub fn dismiss_build_result(&mut self) {
        self.workspaces.build_result = false;
        self.workspaces.build_options = false;
    }
    /// Open or close the build options on the result slip.
    pub fn toggle_build_options(&mut self) {
        self.workspaces.build_options = !self.workspaces.build_options;
        if self.workspaces.build_options {
            self.read_build_targets();
        }
    }
    /// Read the outputs the manifest declares now, keeping the chosen one while it remains.
    fn read_build_targets(&mut self) {
        let Some(project) = self.session.project_mut() else {
            return;
        };
        let (files, _) = project.parts_mut();
        match recite_writer_session::builds::targets(files.root()) {
            Ok(targets) => {
                files.builds.targets = targets;
                if files.builds.selected.as_ref().is_some_and(|selected| {
                    !files
                        .builds
                        .targets
                        .iter()
                        .any(|(asset, _)| asset == selected)
                }) {
                    files.builds.selected = None;
                }
            }
            Err(error) => self.status = error,
        }
    }
    /// The build page from a failed build's result slip: the full report.
    pub fn open_build_page(&mut self) {
        self.open_project_workspace(Open::Build);
    }
}

fn summary(message: &str) -> String {
    message
        .lines()
        .next()
        .unwrap_or("Build finished")
        .chars()
        .take(SUMMARY_CHARS)
        .collect()
}

#[cfg(test)]
mod tests;
