use super::super::SchemaUpdate;
use super::*;
impl Writer {
    pub(super) fn accept_tool(&mut self, mut completed: Completed) -> Retirement {
        let current = self
            .workspaces
            .tools
            .pending
            .as_ref()
            .is_some_and(|request| Arc::ptr_eq(request, &completed.request));
        if !current {
            return Retirement::Drop(completed.result.ok());
        }
        self.workspaces.tools.pending = None;
        let project_matches = self
            .session
            .project()
            .is_some_and(|project| project.root() == completed.request.root);
        if !project_matches {
            self.status = "The project changed while its tool was running.".into();
            return Retirement::Drop(completed.result.ok());
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
                    self.workspaces.declarations.source.invalidate_output();
                    self.workspaces.schema_refresh = Some(SchemaUpdate::Published);
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
                    let update = self.workspaces.schema_refresh.take().map(SchemaUpdate::message).unwrap_or("Schema updated");
                    self.reading.prepare(self.session.model());
                    let stale_rule_draft = self.session.model().has_draft() && self.rules.as_ref().is_some_and(|reply| !reply.projection().belongs_to(self.session.model().document()));
                    if stale_rule_draft { self.retire_rules(); }
                    self.refresh_rules();
                    self.invalidate_map();
                    self.refresh_map_if_needed();
                    self.status = if stale_rule_draft {
                        format!("{update}; project analysis refreshed. The staged reply draft is retained for external repair.")
                    } else { format!("{update}; project analysis refreshed.") };
                }),
            Ok(Value::Declaration(prepared)) => {
                let completion = prepared.completion();
                match self.accept_declaration_tool(prepared) {
                    Ok(work) => {
                        self.workspaces.tools.pending = Some(completed.request);
                        return Retirement::Finish { work: Box::new(work), completion, retired: completed.result.ok() };
                    }
                    Err(error) => Err(error),
                }
            }
            Ok(Value::DeclarationFinished { receipt, completion }) => self.finish_declaration_tool(receipt, *completion),
            Err(error) => Err(error.clone()),
        };
        if let Err(error) = result {
            let message = if completed.request.kind == ToolKind::Refresh
                && let Some(update) = self.workspaces.schema_refresh
            {
                format!("{}; context refresh failed: {error}", update.message())
            } else {
                error
            };
            self.schema_feedback(message);
        }
        Retirement::Drop(completed.result.ok())
    }
}
