//! One destination receipt returns both exclusive owners before checking route freshness.
use super::*;
use crate::{
    localisation::location as catalogue,
    session::{Checkout, RecoveryReason},
    workspaces::location as workspace,
};
use recite_writer_session::location::{self, Screen};

pub struct Plan {
    location: Location,
    catalogue: catalogue::Plan,
    workspace: workspace::Plan,
}
pub struct Work {
    project: Checkout,
    catalogue: catalogue::Work,
    workspace: workspace::Plan,
}
pub struct Prepared {
    project: Checkout,
    catalogue: catalogue::Prepared,
    workspace: Result<workspace::Presentation, String>,
    recovery: Option<RecoveryReason>,
}
impl Plan {
    pub fn capture(
        writer: &Writer,
        location: &Location,
        changed: bool,
        path: Option<PathBuf>,
    ) -> Result<Self, String> {
        if writer.session.read_only() || writer.project_jobs_busy() {
            return Err("Finish the current project action before opening a link.".into());
        }
        let catalogue = catalogue::Plan::capture(writer, location, changed, path)?;
        let workspace = workspace::Plan::capture(writer, location.screen)?;
        Ok(Self {
            location: location.clone(),
            catalogue,
            workspace,
        })
    }
    pub fn needs_work(&self) -> bool {
        self.catalogue.needs_work() || !matches!(self.workspace, workspace::Plan::Writing)
    }
    pub fn checkout(self, writer: &mut Writer) -> Result<Work, String> {
        let mut project = writer
            .session
            .checkout()
            .map_err(|error| error.to_string())?;
        let catalogue = match self.catalogue.checkout(writer) {
            Ok(catalogue) => catalogue,
            Err(error) => {
                writer
                    .session
                    .restore_checkout(&mut project, None)
                    .map_err(|error| error.to_string())?;
                return Err(error);
            }
        };
        Ok(Work {
            project,
            catalogue,
            workspace: self.workspace,
        })
    }
}
impl Work {
    pub fn execute(mut self, control: &dyn WorkControl) -> Prepared {
        let prepared = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let project = self
                .project
                .project
                .as_mut()
                .ok_or("The project owner is unavailable.")?;
            project.checkpoint().map_err(|error| error.to_string())?;
            self.workspace.prepare(project, control)
        }));
        let (workspace, recovery) = match prepared {
            Ok(result) => (result, None),
            Err(_) => (
                Err(
                    "Link preparation stopped unexpectedly. Reopen the project before editing."
                        .into(),
                ),
                Some(RecoveryReason::ReopenRequired),
            ),
        };
        let catalogue = match (&workspace, &self.project.project) {
            (Ok(_), Some(project)) => self.catalogue.prepare(project, control),
            (Err(error), _) => self.catalogue.not_started(error.clone()),
            _ => self
                .catalogue
                .not_started("The project owner is unavailable.".into()),
        };
        Prepared {
            project: self.project,
            catalogue,
            workspace,
            recovery,
        }
    }
    pub fn not_started(self, error: String) -> Prepared {
        Prepared {
            project: self.project,
            catalogue: self.catalogue.not_started(error.clone()),
            workspace: Err(error),
            recovery: None,
        }
    }
}
impl Prepared {
    pub fn restore(&mut self, writer: &mut Writer) -> Result<(), String> {
        // Attempt both independently: one failed freshness proof must never strand the other owner.
        let project = writer
            .session
            .restore_checkout(&mut self.project, self.recovery)
            .map_err(|error| error.to_string());
        let catalogue = self.catalogue.restore(writer);
        project.and(catalogue)
    }
    fn present(&mut self, writer: &mut Writer) -> Result<(), String> {
        self.catalogue.require_ready()?;
        let workspace = self.workspace.as_mut().map_err(|error| error.clone())?;
        writer.present_linked_workspace(workspace)?;
        self.catalogue.present(writer)
    }
}
impl Writer {
    pub fn show_current_catalogue_beat(&mut self, page: usize) -> Result<(), String> {
        if self.phase != crate::Phase::Writing || self.project_action_blocked() {
            return Err("Finish the current action before opening translations.".into());
        }
        self.navigation.require_ready()?;
        let mut location = self.current_location();
        location.screen = Screen::Localise;
        location.source = false;
        location.view = Some(recite_config::WriterView::Script);
        location.beat = location.beat.or_else(|| {
            self.session
                .model()
                .remembered_script_block()
                .map(str::to_owned)
        });
        location.passage = None;
        location.page = page;
        let plan = Plan::capture(self, &location, false, None)?;
        self.remember_location();
        self.start_location_destination(plan, CancellationToken::new())
    }
    pub fn prepare_initial_location_destination(
        &mut self,
        location: &Location,
    ) -> Result<(), String> {
        let path = location
            .catalogue
            .as_ref()
            .map(|path| {
                let project = self
                    .session
                    .project()
                    .ok_or("Open a project before following a catalogue link.")?;
                location::catalogue_path(project.root(), path)
            })
            .transpose()?;
        let changed = path.as_deref() != self.localisation.catalogue_path();
        let plan = Plan::capture(self, location, changed, path)?;
        if plan.needs_work() {
            let mut prepared = plan.checkout(self)?.execute(&CancellationToken::new());
            prepared.restore(self)?;
            prepared.present(self)?;
        }
        self.finish_location_presentation(location, CancellationToken::new())
    }
    pub fn start_location_destination(
        &mut self,
        plan: Plan,
        cancel: CancellationToken,
    ) -> Result<(), String> {
        if !plan.needs_work() {
            return self.finish_location_presentation(&plan.location, cancel);
        }
        self.navigation.require_ready()?;
        let request = Arc::new(Request {
            target: Target::Present(plan.location.clone()),
            origin: Origin::capture(self),
            cancel,
            forwarded: self
                .navigation
                .incoming
                .as_ref()
                .map(|incoming| incoming.cancelled.clone()),
        });
        let work = plan.checkout(self)?;
        self.send_navigation(request, worker::Work::Present(Box::new(work)))
    }
    pub fn finish_location_presentation(
        &mut self,
        location: &Location,
        cancel: CancellationToken,
    ) -> Result<(), String> {
        self.location = location.clone();
        self.reveal_navigation();
        self.trial.visible = location.screen == Screen::Preview;
        self.attention.leave_focus();
        self.cancel_map_gesture();
        self.remember_location();
        if super::location::requests_editor(location) {
            self.queue_linked_editor(cancel)?;
        } else if location.screen == Screen::Write && location.passage.is_some() {
            self.request_editing();
        }
        Ok(())
    }
    pub fn publish_location_destination(
        &mut self,
        request: &Request,
        prepared: &mut Prepared,
    ) -> Result<(), String> {
        self.require_finished_composition()?;
        request.checkpoint().map_err(|error| error.to_string())?;
        if !request.origin.is_current(self) {
            return Err("The document changed while its linked destination was preparing. Your work remains open.".into());
        }
        self.status = "Ready".into();
        prepared.present(self)?;
        if let Target::Present(location) = &request.target {
            self.finish_location_presentation(location, request.cancel.clone())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
