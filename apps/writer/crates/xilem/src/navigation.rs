//! Authoring selection and document navigation use the shared product locations.
use recite_writer_session::location::history as location_history;
mod capture;
mod destination;
mod history;
mod location;
mod opening;
mod publication;
mod reload;
pub(super) mod repair;
mod saving;
pub(super) mod worker;
use super::{Writer, feedback, session::Session};
use recite_compiler::authoring::{CancellationToken, Interrupted, WorkControl};
use recite_writer_model::StructuralCommand;
use recite_writer_model::{AnalysisStamp, EditOperation, RecoveredDraft, View};
use recite_writer_session::FileError;
use recite_writer_session::location::Location;
use std::sync::atomic;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;
pub(super) use worker::worker;

#[derive(Clone)]
pub(super) enum Target {
    Document(PathBuf),
    CompareFile(PathBuf),
    Example(String),
    Location(Location),
    Present(Location),
    CatalogueSource(crate::localisation::SourceIntent),
    Lookup {
        location: Location,
        search: crate::lookup::Search,
    },
    Selection(Location),
    Edit(EditOperation),
    Structure(StructuralCommand),
    Workspace(crate::map::Workspace),
    MapRefresh,
    Reload,
    ProjectWorkspace(crate::workspaces::Open),
    Repair(repair::Action),
}

impl Target {
    fn location(&self) -> Option<&Location> {
        match self {
            Self::Location(location)
            | Self::Lookup { location, .. }
            | Self::Selection(location) => Some(location),
            Self::CatalogueSource(intent) => Some(intent.location()),
            _ => None,
        }
    }
    fn check_scope(&self, writer: &Writer) -> Result<(), String> {
        if let Self::CatalogueSource(intent) = self {
            intent.check(writer)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
pub(super) struct Navigation {
    pub(super) sender: Option<UnboundedSender<worker::Job>>,
    pub(super) runtime: Option<tokio::runtime::Handle>,
    pending: Option<Arc<Request>>,
    incoming: Option<recite_writer_session::activation::IncomingRoute>,
    deferred_save: Option<saving::DeferredSave>,
    pub(super) history: location_history::History,
    visit: Option<location_history::Step>,
}
pub(super) struct Request {
    target: Target,
    origin: Origin,
    cancel: CancellationToken,
    forwarded: Option<Arc<atomic::AtomicBool>>,
}
pub(crate) struct Origin {
    window: xilem::WindowId,
    source: AnalysisStamp,
    draft: RecoveredDraft,
}
impl Navigation {
    pub(crate) fn take_incoming(
        &mut self,
    ) -> Option<recite_writer_session::activation::IncomingRoute> {
        self.incoming.take()
    }
    pub(super) fn new() -> Self {
        Self {
            sender: None,
            runtime: None,
            pending: None,
            incoming: None,
            deferred_save: None,
            history: Default::default(),
            visit: None,
        }
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some() || self.deferred_save.is_some()
    }
}
impl WorkControl for Request {
    fn checkpoint(&self) -> Result<(), Interrupted> {
        self.cancel.checkpoint()?;
        if self
            .forwarded
            .as_ref()
            .is_some_and(|cancelled| cancelled.load(atomic::Ordering::Acquire))
        {
            return Err(Interrupted);
        }
        Ok(())
    }
}
impl Origin {
    pub(crate) fn capture(writer: &Writer) -> Self {
        let model = writer.session.model();
        Self {
            window: writer.window,
            source: model.document().analysis_stamp(),
            draft: model.recovery(),
        }
    }
    fn presentation_is_current(&self, writer: &Writer) -> bool {
        self.window == writer.window
            && self.source.is_current(writer.session.model().document())
            && self.draft.view() == writer.session.model().view()
            && writer.phase == super::Phase::Writing
    }
    pub(crate) fn is_current(&self, writer: &Writer) -> bool {
        self.presentation_is_current(writer) && self.draft == writer.session.model().recovery()
    }
}
impl Writer {
    pub(super) fn switch(&mut self, path: &std::path::Path) {
        self.navigate(Target::Document(path.to_owned()));
    }

    pub(super) fn select_example(&mut self, index: usize) {
        if let Some(example) = recite_writer_model::WRITER_EXAMPLES.get(index) {
            self.navigate(Target::Example(example.name.into()));
        }
    }
    pub(super) fn select(&mut self, id: String) {
        self.navigate(Target::Selection(Location {
            screen: recite_writer_session::location::Screen::Write,
            passage: Some(id),
            source: false,
            view: Some(recite_config::WriterView::Script),
            ..self.current_location()
        }));
    }
    pub(super) fn inspect(&mut self, id: String) {
        self.navigate(Target::Selection(Location {
            screen: recite_writer_session::location::Screen::Write,
            beat: Some(id),
            passage: None,
            source: false,
            view: Some(recite_config::WriterView::Script),
            ..self.current_location()
        }));
    }
    pub(super) fn read_current(&mut self) {
        match self.session.model().selected_block() {
            Ok(Some(id)) => self.inspect(id),
            Ok(None) => self.status = "Choose a beat in the outline.".into(),
            Err(error) => self.status = feedback::workbench(&error),
        }
    }
    pub(super) fn navigate(&mut self, target: Target) {
        if let Err(error) = self.begin_navigation(target) {
            self.status = error;
        }
    }
    pub(super) fn navigate_activation(
        &mut self,
        location: Location,
        incoming: recite_writer_session::activation::IncomingRoute,
    ) -> Result<(), String> {
        if self.work.busy() || self.navigation.busy() || self.phase != super::Phase::Writing {
            return Err("Finish the current editor action before opening a link.".into());
        }
        self.navigation.incoming = Some(incoming);
        if let Err(error) = self.begin_navigation(Target::Location(location)) {
            self.finish_navigation_reply(Err(error.clone()));
            return Err(error);
        }
        Ok(())
    }
    fn finish_navigation_reply(&mut self, result: Result<(), String>) {
        if let Some(incoming) = self.navigation.incoming.take() {
            if result.is_ok()
                && let Some(editor) = incoming.editor
            {
                self.editor = Some(editor);
            }
            let _ = incoming.reply.send(result);
        }
    }
    pub(super) fn begin_navigation(&mut self, target: Target) -> Result<(), String> {
        if self.session.read_only() {
            return Err(FileError::ProjectActionPending.to_string());
        }
        if self.work.busy() || self.navigation.busy() || self.phase != super::Phase::Writing {
            return Err("Finish the current editor action before navigating.".into());
        }
        if matches!(&target, Target::Document(path) if self.session.project().is_some_and(|p| p.current().path() == path))
            || matches!(&target, Target::Example(name) if self.session.model().document().key().as_str() == name)
        {
            return Ok(());
        }
        if self
            .navigation
            .sender
            .as_ref()
            .is_none_or(UnboundedSender::is_closed)
            || self.navigation.runtime.is_none()
        {
            return Err("The navigation worker is unavailable. Reopen Writer to retry.".into());
        }
        self.cancel_map_gesture();
        self.trial.cancel();
        if self.source_repair_pending()
            && matches!(
                target,
                Target::Location(_)
                    | Target::CatalogueSource(_)
                    | Target::Lookup { .. }
                    | Target::Edit(_)
                    | Target::Structure(_)
                    | Target::Workspace(_)
            )
        {
            return Err("Repair and import the recovered source draft before applying it or following a link. You can switch documents to keep this draft for later.".into());
        }
        if self.navigation.visit.is_none() {
            self.navigation.history.record(self.current_location());
        }
        let applies = (matches!(
            target,
            Target::Location(_)
                | Target::CatalogueSource(_)
                | Target::Lookup { .. }
                | Target::Edit(_)
                | Target::Structure(_)
                | Target::Workspace(_)
        ) || (matches!(target, Target::Selection(_))
            && matches!(self.session.model().view(), View::Passage(_))))
            && self.session.model().has_draft();
        if applies {
            self.submit_edit(target, EditOperation::Apply)
        } else if let Target::Edit(operation) = target {
            self.submit_edit(Target::Edit(operation), operation)
        } else if let Target::Structure(command) = target {
            self.submit_structural(command)
        } else {
            self.submit_navigation(target)
        }
    }
    pub(super) fn cancel_navigation(&mut self) {
        let queued_save = self.navigation.deferred_save.take().is_some();
        if let Some(request) = &self.navigation.pending {
            request.cancel.interrupt();
            self.status = "Cancelling navigation; your current work stays open.".into();
        }
        if queued_save {
            self.status = "Requested save cancelled; your work stays open.".into();
        }
    }
    pub(crate) fn cancel_map_preparation(&mut self) {
        if let Some(request) = &self.navigation.pending
            && matches!(request.target, Target::Workspace(_) | Target::MapRefresh)
        {
            request.cancel.interrupt();
        }
    }
    pub(crate) fn cancel_project_workspace_open(&mut self) {
        if self
            .navigation
            .pending
            .as_ref()
            .is_some_and(|request| matches!(request.target, Target::ProjectWorkspace(_)))
        {
            self.cancel_navigation();
        }
    }
    fn send_navigation(&mut self, request: Arc<Request>, work: worker::Work) -> Result<(), String> {
        let runtime = self
            .navigation
            .runtime
            .as_ref()
            .cloned()
            .ok_or("The navigation worker is starting. Retry this action in a moment.")?;
        let result = self
            .navigation
            .sender
            .as_ref()
            .ok_or("The navigation worker is unavailable. Reopen Writer to retry.")?
            .send(worker::Job {
                request: request.clone(),
                work,
            });
        if let Err(error) = result {
            error.0.request.cancel.interrupt();
            if let worker::Work::Present(work) = error.0.work {
                let mut prepared = work
                    .not_started("The navigation worker stopped. Reopen Writer to retry.".into());
                let restored = prepared.restore(self);
                runtime.spawn_blocking(move || drop(prepared));
                let error = restored.err().unwrap_or_else(|| {
                    "The navigation worker stopped. Reopen Writer to retry.".into()
                });
                self.navigation_failed(error);
                return Err(self.status.clone());
            }
            // The worker stays mounted during authoring. Still retire a rejected capture
            // through its runtime, including the transferred recovery fence.
            runtime.spawn_blocking(move || error.0.retire());
            self.navigation_failed("The navigation worker stopped. Reopen Writer to retry.".into());
            return Err(self.status.clone());
        }
        self.status = match &request.target {
            Target::Repair(repair::Action::Export) => "Exporting the captured source draft…",
            Target::Repair(repair::Action::Import(_)) => {
                "Checking repaired source… Your current draft stays open."
            }
            Target::Repair(repair::Action::Choose) => {
                "Choose the source file repaired in your editor."
            }
            Target::Edit(EditOperation::Apply) if self.rules.is_some() => {
                "Applying reply draft… Controls will reopen when it finishes."
            }
            Target::Edit(EditOperation::Apply) => "Applying draft… You can keep editing.",
            Target::Edit(EditOperation::Undo) => "Undoing applied edit… You can keep editing.",
            Target::Edit(EditOperation::Redo) => "Redoing applied edit… You can keep editing.",
            Target::Structure(_) => "Preparing structural edit… Your current work stays open.",
            Target::Workspace(_) => "Preparing scene map… Your current work stays open.",
            Target::MapRefresh => "Updating scene map… You can keep editing.",
            Target::Reload => "Preparing disk reload… Your current work stays open.",
            _ => "Preparing navigation… You can keep editing.",
        }
        .into();
        self.navigation.pending = Some(request);
        Ok(())
    }
}
