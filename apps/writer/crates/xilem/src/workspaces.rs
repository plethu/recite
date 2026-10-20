//! Project screens retain writing widgets; durable state remains in the shared project owner.
mod builds;
mod declarations;
mod opening;
mod text;
use crate::{Phase, Writer};
pub(crate) use opening::{Open, Prepared, Work};
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
}
#[derive(Default)]
pub(crate) struct Workspaces {
    pub screen: Screen,
    declarations: declarations::Panel,
    build_page: usize,
    build_feedback: Vec<String>,
    feedback_page: usize,
}
impl Writer {
    pub(crate) fn project_jobs_busy(&self) -> bool {
        self.session.project().is_some_and(|project| {
            project.files().builds.busy()
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
        let retained = self.session.project().is_some_and(|project| match action {
            Open::Declarations => project.files().declarations.is_some(),
            Open::Build => project.files().builds.busy(),
        });
        if retained && self.phase == Phase::Writing && !self.navigation.busy() && !self.work.busy()
        {
            self.attention.leave_focus();
            self.cancel_map_gesture();
            self.workspaces.screen = match action {
                Open::Declarations => Screen::Declarations,
                Open::Build => Screen::Build,
            };
            return;
        }
        self.navigate(crate::navigation::Target::ProjectWorkspace(action));
    }
}

pub(crate) fn panel(writer: &Writer) -> impl xilem::WidgetView<Writer> + use<> {
    xilem::view::indexed_stack((declarations::panel(writer), builds::panel(writer)))
        .active(usize::from(writer.workspaces.screen == Screen::Build))
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
