//! Project failure recovery keeps technical diagnostics behind explicit disclosure.
pub use super::Shell;

pub enum ProjectAction {
    Open,
    Create,
}
pub struct Problem {
    action: ProjectAction,
    pub diagnostic: String,
    pub expanded: bool,
}
impl Problem {
    pub fn heading(&self) -> &'static str {
        match self.action {
            ProjectAction::Open => "Project didn’t open",
            ProjectAction::Create => "Project wasn’t created",
        }
    }
    pub fn guidance(&self) -> &'static str {
        match self.action {
            ProjectAction::Open => {
                "Choose the folder containing recite.project.toml, or create a new project."
            }
            ProjectAction::Create => {
                "Choose a writable folder and a project name. Existing project files are preserved."
            }
        }
    }
}
impl Shell {
    pub(crate) fn clear_project_problem(&mut self) {
        if self.welcome_problem.take().is_some() {
            self.status.clear();
        }
    }
    pub fn project_failed(&mut self, action: ProjectAction, diagnostic: String) {
        self.status = diagnostic.clone();
        self.welcome_problem = Some(Problem {
            action,
            diagnostic,
            expanded: false,
        });
    }
}
