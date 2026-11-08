//! Project screens retain writing widgets; durable state remains in the shared project owner.
pub mod builds;
pub mod declarations;
pub mod external;
pub mod location;
pub mod opening;
pub mod rename;
pub use rename::BeatChoice;
pub mod settings;
pub mod text;
pub mod tools;
use crate::{Phase, Writer};
pub use opening::{Open, Prepared, Work};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Write,
    Declarations,
    Build,
    Settings,
    Rename,
    External,
    Localisation,
}
#[derive(Clone, Copy)]
pub enum ContextUpdate {
    Published,
    Reloaded,
    SettingsSaved,
    SettingsReloaded,
    SettingsUnconfirmed,
}
impl ContextUpdate {
    fn message(self) -> &'static str {
        match self {
            Self::Published => "Schema published",
            Self::Reloaded => "Schema reloaded",
            Self::SettingsSaved => "Project settings saved",
            Self::SettingsReloaded => "Project settings reloaded",
            Self::SettingsUnconfirmed => "Saved project settings need confirmation",
        }
    }
}
#[derive(Default)]
pub struct Workspaces {
    pub screen: Screen,
    pub declarations: declarations::Panel,
    pub settings: settings::Panel,
    pub rename: rename::Panel,
    pub external: external::Panel,
    pub build_page: usize,
    pub build_feedback: Vec<String>,
    pub feedback_page: usize,
    pub tools: tools::Tools,
    pub context_refresh: Option<ContextUpdate>,
    pub schema_feedback: Vec<String>,
    pub schema_feedback_page: usize,
}
impl Writer {
    pub fn invalidate_declaration_output(&mut self) {
        self.workspaces.declarations.source.invalidate_output();
    }
    pub fn project_jobs_busy(&self) -> bool {
        self.session.checked_out()
            || self.localisation.working()
            || self.workspaces.tools.busy()
            || self.session.project().is_some_and(|project| {
                project.files().builds.busy()
                    || project
                        .files()
                        .settings
                        .as_ref()
                        .is_some_and(|session| session.busy())
                    || project
                        .files()
                        .declarations
                        .as_ref()
                        .is_some_and(|session| session.busy())
            })
    }
    pub fn return_to_writing(&mut self) {
        if self.phase != Phase::Writing {
            return;
        }
        self.cancel_project_workspace_open();
        self.cancel_history_navigation();
        self.remember_location();
        self.workspaces.screen = Screen::Write;
        self.return_from_trial();
        self.refresh_map_if_needed();
        self.remember_location();
    }
    pub fn open_project_workspace(&mut self, action: Open) {
        if matches!(action, Open::Rename) {
            self.workspaces
                .rename
                .choose_block(self.session.model().selected_block().ok().flatten());
        }
        let retained = self.session.project().is_some_and(|project| match action {
            Open::Declarations => project.files().declarations.is_some(),
            Open::Build => project.files().builds.busy(),
            Open::Settings => project.files().settings.is_some(),
            Open::Rename => true,
        });
        if retained && self.phase == Phase::Writing && !self.navigation.busy() && !self.work.busy()
        {
            self.attention.leave_focus();
            self.cancel_map_gesture();
            self.remember_location();
            self.workspaces.screen = match action {
                Open::Declarations => Screen::Declarations,
                Open::Build => Screen::Build,
                Open::Settings => Screen::Settings,
                Open::Rename => Screen::Rename,
            };
            self.remember_location();
            return;
        }
        self.navigate(crate::navigation::Target::ProjectWorkspace(action));
    }
}

#[cfg(any(test, feature = "test-support"))]
pub mod tests;
