//! Project screens retain writing widgets; durable state remains in the shared project owner.
mod builds;
mod declarations;
mod external;
mod opening;
mod rename;
pub(crate) use rename::recovery;
mod settings;
mod text;
mod tools;
use crate::{Phase, Writer};
pub(crate) use opening::{Open, Prepared, Work};
pub(crate) use tools::worker;
use xilem::{
    ViewCtx,
    core::{NoElement, ViewSequence},
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Screen {
    #[default]
    Write,
    Declarations,
    Build,
    Settings,
    Rename,
    External,
}
#[derive(Clone, Copy)]
enum ContextUpdate {
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
pub(crate) struct Workspaces {
    pub screen: Screen,
    declarations: declarations::Panel,
    settings: settings::Panel,
    rename: rename::Panel,
    external: external::Panel,
    build_page: usize,
    build_feedback: Vec<String>,
    feedback_page: usize,
    tools: tools::Tools,
    context_refresh: Option<ContextUpdate>,
    schema_feedback: Vec<String>,
    schema_feedback_page: usize,
}
impl Writer {
    pub(crate) fn invalidate_declaration_output(&mut self) {
        self.workspaces.declarations.source.invalidate_output();
    }
    pub(crate) fn project_jobs_busy(&self) -> bool {
        self.session.checked_out()
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
    pub(crate) fn return_to_writing(&mut self) {
        if self.phase != Phase::Writing {
            return;
        }
        self.cancel_project_workspace_open();
        self.workspaces.screen = Screen::Write;
        self.refresh_map_if_needed();
    }
    pub(crate) fn open_project_workspace(&mut self, action: Open) {
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
            self.workspaces.screen = match action {
                Open::Declarations => Screen::Declarations,
                Open::Build => Screen::Build,
                Open::Settings => Screen::Settings,
                Open::Rename => Screen::Rename,
            };
            return;
        }
        self.navigate(crate::navigation::Target::ProjectWorkspace(action));
    }
}

pub(crate) fn panel(writer: &Writer) -> impl xilem::WidgetView<Writer> + use<> {
    xilem::view::indexed_stack((
        declarations::panel(writer),
        builds::panel(writer),
        settings::panel(writer),
        rename::panel(writer),
        external::panel(writer),
    ))
    .active(match writer.workspaces.screen {
        Screen::Build => 1,
        Screen::Settings => 2,
        Screen::Rename => 3,
        Screen::External => 4,
        Screen::Write | Screen::Declarations => 0,
    })
}

// Conditional task lifetime follows active jobs, independently of the selected screen.
pub(crate) fn jobs(writer: &Writer) -> impl ViewSequence<Writer, (), ViewCtx, NoElement> + use<> {
    writer.project_jobs_busy().then(|| {
        xilem::view::task(
            |proxy, _: &mut Writer| async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    if proxy.message(()).is_err() {
                        break;
                    }
                }
            },
            |writer: &mut Writer, ()| writer.poll_project_jobs(),
        )
    })
}

#[cfg(test)]
mod tests;
