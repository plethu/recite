use super::*;
use recite_writer_session::navigation::{Destination, Prepare};

impl Writer {
    pub(super) fn submit_structural(&mut self, command: StructuralCommand) -> Result<(), String> {
        let edit = self
            .session
            .model()
            .structural_request(command.clone())
            .map_err(|error| error.to_string())?;
        let request = Arc::new(Request {
            target: Target::Structure(command),
            origin: Origin::capture(self),
            cancel: CancellationToken::new(),
            forwarded: None,
        });
        self.send_navigation(request, worker::Work::Structure(Box::new(edit)))
    }
    pub(super) fn submit_edit(
        &mut self,
        target: Target,
        operation: EditOperation,
    ) -> Result<(), String> {
        let edit = self
            .session
            .model()
            .edit_request(operation)
            .map_err(|error| error.to_string())?;
        let Some(edit) = edit else {
            self.status = "No applied change for this command.".into();
            return Ok(());
        };
        let edit = if operation == EditOperation::Apply
            && let Some(rules) = &self.rules
        {
            edit.with_rules(rules.projection().clone())
        } else {
            edit
        };
        let request = Arc::new(Request {
            target,
            origin: Origin::capture(self),
            cancel: CancellationToken::new(),
            forwarded: self
                .navigation
                .incoming
                .as_ref()
                .map(|incoming| incoming.cancelled.clone()),
        });
        self.send_navigation(
            request,
            worker::Work::Edit {
                edit: Box::new(edit),
                operation,
            },
        )
    }
    pub(super) fn submit_navigation(&mut self, target: Target) -> Result<(), String> {
        if self
            .navigation
            .sender
            .as_ref()
            .is_none_or(UnboundedSender::is_closed)
            || self.navigation.runtime.is_none()
        {
            return Err("The navigation worker is unavailable. Reopen Writer to retry.".into());
        }
        if matches!(target, Target::Workspace(_) | Target::MapRefresh) {
            let work = self.prepare_map(if matches!(target, Target::Workspace(_)) {
                crate::map::Purpose::Workspace
            } else {
                crate::map::Purpose::Placement
            });
            let request = Arc::new(Request {
                target,
                origin: Origin::capture(self),
                cancel: CancellationToken::new(),
                forwarded: None,
            });
            return self.send_navigation(request, worker::Work::Map(Box::new(work)));
        }
        if matches!(target, Target::Reload) {
            return self.submit_reload();
        }
        if let Target::ProjectWorkspace(action) = target {
            let work = crate::workspaces::Work::capture(self, action)?;
            let request = Arc::new(Request {
                target,
                origin: Origin::capture(self),
                cancel: CancellationToken::new(),
                forwarded: None,
            });
            return self.send_navigation(request, worker::Work::ProjectWorkspace(work));
        }
        let model = self.session.model();
        let location = match &target {
            Target::Location(location)
            | Target::Lookup { location, .. }
            | Target::Selection(location) => Some(location.clone()),
            _ => None,
        };
        let destination = match &target {
            Target::Document(path) => Destination::Switch(Box::new(
                self.session
                    .project()
                    .ok_or("Open a project first.")?
                    .files()
                    .switch_request(model, path, None)
                    .map_err(|error| error.to_string())?,
            )),
            Target::Example(name) => {
                let Session::Examples { sessions, .. } = &self.session else {
                    return Err("Open temporary examples first.".into());
                };
                Destination::Example(Box::new(
                    sessions
                        .request(model, name)
                        .map_err(|error| error.to_string())?,
                ))
            }
            Target::Location(location)
            | Target::Lookup { location, .. }
            | Target::Selection(location) => self.location_destination(location)?,
            Target::Edit(_)
            | Target::Repair(_)
            | Target::Structure(_)
            | Target::Workspace(_)
            | Target::Reload
            | Target::ProjectWorkspace(_)
            | Target::MapRefresh => {
                return Err("Invalid navigation target.".into());
            }
        };
        let request = Arc::new(Request {
            target,
            origin: Origin::capture(self),
            cancel: CancellationToken::new(),
            forwarded: self
                .navigation
                .incoming
                .as_ref()
                .map(|incoming| incoming.cancelled.clone()),
        });
        let prepare = Prepare {
            destination,
            location,
            project: self
                .session
                .project()
                .map(|project| project.root().to_owned()),
            catalogue: None,
            reuse_rules: false,
            initial_view: None,
        };
        let checkpoint = if matches!(prepare.destination, Destination::Switch(_)) {
            let (files, model) = self
                .session
                .parts_mut()
                .ok_or("Project closed before navigation.")?;
            Some(Box::new(
                files
                    .begin_navigation_checkpoint(model)
                    .map_err(|error| error.to_string())?,
            ))
        } else {
            None
        };
        self.send_navigation(
            request,
            worker::Work::Navigate {
                prepare: Box::new(prepare),
                checkpoint,
            },
        )
    }
    fn location_destination(&self, location: &Location) -> Result<Destination, String> {
        let model = self.session.model();
        if location.document.is_empty() || location.document == model.document().key().as_str() {
            return Ok(Destination::Current(Box::new(model.navigation_request())));
        }
        match &self.session {
            Session::Project(project) => {
                let path = project
                    .files()
                    .path_for_document(&location.document)
                    .ok_or("The linked document is not in this project.")?;
                Ok(Destination::Switch(Box::new(
                    project
                        .files()
                        .switch_request(model, &path, None)
                        .map_err(|error| error.to_string())?,
                )))
            }
            Session::Examples { sessions, .. } => Ok(Destination::Example(Box::new(
                sessions
                    .request(model, &location.document)
                    .map_err(|error| error.to_string())?,
            ))),
            Session::Working { .. } | Session::Recovering { .. } => {
                Err(FileError::ProjectActionPending.to_string())
            }
        }
    }
}
