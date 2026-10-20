use super::*;
use recite_writer_session::navigation::PreparedDestination;
use worker::{Completed, ResultValue};

impl Writer {
    pub(crate) fn accept_navigation(&mut self, completed: Completed) -> Option<ResultValue> {
        let Completed { request, result } = completed;
        if !self
            .navigation
            .pending
            .as_ref()
            .is_some_and(|pending| Arc::ptr_eq(pending, &request))
        {
            return result.ok();
        }
        self.navigation.pending = None;
        let mut value = match result {
            Ok(value) => value,
            Err(error) => {
                self.navigation_failed(error);
                return None;
            }
        };
        let result = match &mut value {
            ResultValue::Repair(repaired) => self.publish_source_repair(&request, repaired),
            _ if request.checkpoint().is_err() => {
                Err("Navigation cancelled; your current work stays open.".into())
            }
            ResultValue::Edit {
                prepared,
                operation,
            } => self.publish_navigation_edit(&request, prepared, *operation),
            ResultValue::Navigation {
                prepared,
                checkpoint,
            } => self.publish_navigation(&request, prepared, checkpoint.as_ref()),
        };
        if let Err(error) = result {
            self.navigation_failed(error);
        } else {
            self.mark_deferred_save_applied(&request);
            if !self.navigation.busy() {
                self.finish_navigation_reply(Ok(()));
            }
        }
        Some(value)
    }
    fn publish_navigation_edit(
        &mut self,
        request: &Request,
        prepared: &mut recite_writer_model::PreparedEdit,
        operation: EditOperation,
    ) -> Result<(), String> {
        if !request.origin.presentation_is_current(self) {
            return Err(
                "The document changed while the command was preparing. Your work stays open."
                    .into(),
            );
        }
        self.session
            .edit(|model| model.accept_edit(prepared))
            .map_err(|error| error.to_string())?;
        self.reading.prepare(self.session.model());
        if self.session.model().has_draft() {
            let message =
                "Applied captured text; newer typing remains open. Retry navigation when ready.";
            if matches!(request.target, Target::Edit(EditOperation::Apply)) {
                self.status = message.into();
                return Ok(());
            }
            return Err(message.into());
        }
        match &request.target {
            Target::Edit(target) if *target == operation => {
                self.status = "Applied edit updated.".into();
                Ok(())
            }
            Target::Edit(target) => self.submit_edit(request.target.clone(), *target),
            target => self.submit_navigation(target.clone()),
        }
    }
    fn publish_navigation(
        &mut self,
        request: &Request,
        prepared: &mut recite_writer_session::navigation::Prepared,
        checkpoint: Option<&recite_writer_session::project::ReadyCheckpoint>,
    ) -> Result<(), String> {
        if !request.origin.is_current(self) {
            return Err("The draft changed while navigation was preparing. Your edits remain open; retry navigation.".into());
        }
        match (&mut prepared.destination, &mut self.session) {
            (PreparedDestination::Current(selection), session) => session
                .edit(|model| model.accept_navigation(selection))
                .map_err(|error| error.to_string())?,
            (PreparedDestination::Switch(target), Session::Project(project)) => {
                let (files, model) = project.parts_mut();
                files
                    .accept_switch(
                        model,
                        target,
                        checkpoint.ok_or("Missing outgoing recovery checkpoint.")?,
                    )
                    .map_err(|error| error.to_string())?;
            }
            (PreparedDestination::Example(target), Session::Examples { sessions, model }) => {
                sessions
                    .accept(model, target)
                    .map_err(|error| error.to_string())?
            }
            _ => return Err("The navigation destination is no longer available.".into()),
        }
        if let Target::Location(location) = &request.target {
            self.location = location.clone();
            self.trial.visible =
                location.screen == recite_writer_session::location::Screen::Preview;
            std::mem::swap(&mut self.rules, &mut prepared.rules);
        } else if let Target::Selection(location) = &request.target {
            self.location.document = location.document.clone();
            self.location.beat = location.beat.clone();
            self.location.passage = location.passage.clone();
            self.location.source = location.source;
            self.location.view = location.view;
        }
        self.beat_page = 0;
        self.remember_location();
        self.reading.prepare(self.session.model());
        self.status = if self.session.dirty() {
            "Unsaved work restored"
        } else {
            "Ready"
        }
        .into();
        Ok(())
    }
    pub(super) fn navigation_failed(&mut self, error: String) {
        self.navigation.visit = None;
        // Parking may have completed before cancellation or failed destination validation.
        if let Some((files, model)) = self.session.parts_mut() {
            let _ = files.queue_checkpoint(model);
        }
        self.status = if self.navigation.deferred_save.take().is_some() {
            format!("{error} Requested save was not run.")
        } else {
            error.clone()
        };
        self.finish_navigation_reply(Err(error));
    }
}
