use super::*;
impl Writer {
    pub(super) fn accept_tool(&mut self, mut completed: Completed) -> Option<Value> {
        let current = self
            .workspaces
            .tools
            .pending
            .as_ref()
            .is_some_and(|request| Arc::ptr_eq(request, &completed.request));
        if !current {
            return completed.result.ok();
        }
        self.workspaces.tools.pending = None;
        let project_matches = self
            .session
            .project()
            .is_some_and(|project| project.root() == completed.request.root);
        if !project_matches {
            self.status = "The project changed while its tool was running.".into();
            return completed.result.ok();
        }
        let result = match &mut completed.result {
            Ok(Value::Started(receipt)) => self
                .declaration_session_mut()
                .ok_or("Declarations were closed.".to_owned())
                .and_then(|session| session.accept_generation_start(receipt))
                .map(|()| self.status = "Generating schema… You can keep writing.".into()),
            Ok(Value::Published { receipt, rows }) => self
                .declaration_session_mut()
                .ok_or("Declarations were closed.".to_owned())
                .and_then(|session| session.accept_generation(receipt))
                .map(|()| {
                    self.workspaces.declarations.replace(std::mem::take(rows));
                    self.workspaces.schema_needs_refresh = true;
                    self.refresh_schema_context();
                }),
            Ok(Value::Refreshed(prepared)) => self
                .session
                .project_mut()
                .ok_or("The project was closed.".to_owned())
                .and_then(|project| {
                    project
                        .accept_refresh(prepared)
                        .map_err(|error| error.to_string())
                })
                .map(|()| {
                    self.workspaces.schema_needs_refresh = false;
                    self.reading.prepare(self.session.model());
                    let stale_rule_draft = self.session.model().has_draft() && self.rules.as_ref().is_some_and(|reply| !reply.projection().belongs_to(self.session.model().document()));
                    if stale_rule_draft { self.retire_rules(); }
                    self.refresh_rules();
                    self.invalidate_map();
                    self.refresh_map_if_needed();
                    self.status = if stale_rule_draft {
                        "Schema published; project analysis refreshed. The staged reply draft is retained for external repair."
                    } else { "Schema published; project analysis refreshed." }.into();
                }),
            Err(error) => Err(error.clone()),
        };
        if let Err(error) = result {
            let message = if self.workspaces.schema_needs_refresh {
                format!("Schema published; context refresh failed: {error}")
            } else {
                error
            };
            self.schema_feedback(message);
        }
        completed.result.ok()
    }
}
