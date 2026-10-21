use super::*;
use recite_writer_session::navigation::PreparedDestination;
use worker::{Completed, ResultValue};

impl Writer {
    pub(crate) fn accept_navigation(&mut self, completed: Completed) -> Option<ResultValue> {
        let Completed {
            request,
            mut result,
        } = completed;
        if let Ok(ResultValue::Reload(prepared)) = &result {
            self.record_reload_archive(prepared);
        }
        let restored = match &mut result {
            Ok(ResultValue::Presented(prepared)) => prepared.restore(self),
            _ => Ok(()),
        };
        if !self
            .navigation
            .pending
            .as_ref()
            .is_some_and(|pending| Arc::ptr_eq(pending, &request))
        {
            if let Ok(ResultValue::Map(prepared)) = &mut result {
                self.reconcile_map_placement(prepared);
            }
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
        let result = restored.and_then(|()| match &mut value {
            ResultValue::Repair(repaired) => self.publish_source_repair(&request, repaired),
            _ if request.checkpoint().is_err() => {
                Err("Navigation cancelled; your current work stays open.".into())
            }
            ResultValue::ProjectWorkspace(prepared) => {
                if request.origin.window != self.window || self.phase != crate::Phase::Writing {
                    Err("The project window changed while its workspace was opening.".into())
                } else {
                    self.accept_project_workspace(prepared)
                }
            }
            ResultValue::Presented(prepared) => {
                self.publish_location_destination(&request, prepared)
            }
            ResultValue::Map(prepared) => {
                let current = match request.target {
                    Target::MapRefresh => request.origin.presentation_is_current(self),
                    _ => request.origin.is_current(self),
                };
                if !current {
                    Err("The scene changed while its map was preparing.".into())
                } else {
                    match request.target {
                        Target::Workspace(workspace) => self.accept_map(prepared, workspace),
                        Target::MapRefresh => self.accept_map(prepared, self.map.workspace),
                        _ => Err("Invalid map request.".into()),
                    }
                }
            }
            ResultValue::Edit {
                prepared,
                operation,
            } => self.publish_navigation_edit(&request, prepared, *operation),
            ResultValue::Structure(prepared) => self.publish_structural_edit(&request, prepared),
            ResultValue::Reload(prepared) => self.publish_reload(&request, prepared),
            ResultValue::Navigation {
                prepared,
                checkpoint,
            } => self.publish_navigation(&request, prepared, checkpoint.as_ref()),
        });
        if let Err(error) = result {
            if let ResultValue::Map(prepared) = &mut value {
                self.reconcile_map_placement(prepared);
            }
            self.navigation_failed(error);
            if matches!(request.target, Target::Present(_)) {
                self.remember_location();
            }
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
        request.target.check_scope(self)?;
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
        self.invalidate_map();
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
                if operation != EditOperation::Apply {
                    self.reset_rule_structure();
                }
                if self.navigation.deferred_save.is_none() {
                    self.refresh_rules();
                }
                Ok(())
            }
            Target::Edit(target) => self.submit_edit(request.target.clone(), *target),
            Target::Structure(command) => self.submit_structural(command.clone()),
            target => self.submit_navigation(target.clone()),
        }
    }
    fn publish_structural_edit(
        &mut self,
        request: &Request,
        prepared: &mut recite_writer_model::PreparedStructuralEdit,
    ) -> Result<(), String> {
        if !request.origin.is_current(self) {
            return Err("The draft changed while the structural edit was preparing. Your current work stays open.".into());
        }
        self.session
            .edit(|model| model.accept_structural_edit(prepared))
            .map_err(|error| error.to_string())?;
        self.retire_rules();
        self.reading.prepare(self.session.model());
        self.sync_map_selection();
        self.invalidate_map();
        self.remember_location();
        self.status = "Structural edit updated. Save when ready.".into();
        self.workspaces.screen = crate::workspaces::Screen::Write;
        Ok(())
    }
    fn publish_navigation(
        &mut self,
        request: &Request,
        prepared: &mut recite_writer_session::navigation::Prepared,
        checkpoint: Option<&recite_writer_session::project::ReadyCheckpoint>,
    ) -> Result<(), String> {
        request.target.check_scope(self)?;
        if !request.origin.is_current(self) {
            return Err("The draft changed while navigation was preparing. Your edits remain open; retry navigation.".into());
        }
        let destination = match &request.target {
            Target::Location(location) => Some(destination::Plan::capture(
                self,
                location,
                prepared.catalogue_changed,
                prepared.catalogue.clone(),
            )?),
            _ => None,
        };
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
        if let Target::Selection(location) = &request.target {
            self.retire_rules();
            self.location.document = location.document.clone();
            self.location.beat = location.beat.clone();
            self.location.passage = location.passage.clone();
            self.location.source = location.source;
            self.location.view = location.view;
        } else if let Some(location) = request.target.location() {
            self.location = location.clone();
            self.trial.visible =
                location.screen == recite_writer_session::location::Screen::Preview;
            self.install_rules(&mut prepared.rules);
            if let Some(view) = location.view {
                self.map.workspace = if view == recite_config::WriterView::Map {
                    if self.config.writer.presentation.split() {
                        crate::map::Workspace::Split
                    } else {
                        crate::map::Workspace::Map
                    }
                } else {
                    crate::map::Workspace::Script
                };
            }
        } else {
            self.retire_rules();
        }
        if let Target::CatalogueSource(intent) = &request.target {
            intent.complete(self);
        }
        self.beat_page = 0;
        self.workspaces.screen = crate::workspaces::Screen::Write;
        if destination.is_none() {
            self.remember_location();
        }
        self.reading.prepare(self.session.model());
        self.sync_map_selection();
        self.invalidate_map();
        if let Target::Lookup { search, .. } = &request.target {
            self.accept_lookup(search);
        }
        self.status = if let Some(status) = self
            .rules
            .as_mut()
            .and_then(crate::rules::ReplySession::take_refresh_status)
        {
            status
        } else if self.session.dirty() {
            "Unsaved work restored".into()
        } else {
            "Ready".into()
        };
        if matches!(request.target, Target::CompareFile(_)) {
            self.compare_disk_changes();
        }
        if let Some(destination) = destination {
            self.start_location_destination(destination, request.cancel.clone())?;
        }
        Ok(())
    }
    pub(crate) fn navigation_failed(&mut self, error: String) {
        self.navigation.visit = None;
        if self
            .rules
            .as_mut()
            .and_then(crate::rules::ReplySession::take_refresh_status)
            .is_some()
        {
            self.retire_rules();
            self.location.passage = None;
        }
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
