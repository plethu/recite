//! Checked source adoption and its recovery fence share one worker continuation.
use super::super::SchemaUpdate;
use super::*;
use crate::external_editor::Request as EditorRequest;
use recite_writer_session::declarations::SourceStatus;
pub(super) struct Prepared {
    receipt: SourceReceipt,
    preview: Vec<String>,
    rows: Option<Vec<Row>>,
    validation: Result<SourceStatus, String>,
}
impl Prepared {
    pub(super) fn new(receipt: SourceReceipt) -> Self {
        let preview = receipt
            .draft()
            .map(super::super::text::pages)
            .unwrap_or_default();
        let rows = receipt
            .last_valid_schema()
            .map(|schema| entries::entries(schema).into_iter().map(Row::new).collect());
        let validation = receipt.inspect_source().map_err(|error| error.to_string());
        Self {
            receipt,
            preview,
            rows,
            validation,
        }
    }
}
impl Writer {
    pub(crate) fn open_declaration_source(&mut self) {
        if self.external.busy() || self.declaration_action_blocked() {
            return;
        }
        let Some(project) = self.session.project() else {
            return;
        };
        let Some(source) = project
            .files()
            .declarations
            .as_ref()
            .and_then(|session| session.source.as_ref())
        else {
            return;
        };
        let request =
            EditorRequest::declaration_source(project, source.path.clone(), self.editor.clone());
        self.launch_editor(Arc::new(request));
    }
    pub(crate) fn open_declaration_draft(&mut self) {
        if self.external.busy() || self.declaration_action_blocked() {
            return;
        }
        let Some(project) = self.session.project() else {
            return;
        };
        let Some(path) = self.workspaces.declarations.source.exported() else {
            return;
        };
        let request =
            EditorRequest::declaration_draft(project, path.to_owned(), self.editor.clone());
        self.launch_editor(Arc::new(request));
    }
    pub(crate) fn source_action(&mut self, action: SourceAction) {
        if self.declaration_action_blocked() {
            return;
        }
        self.workspaces.schema_feedback.clear();
        self.workspaces.schema_feedback_page = 0;
        let result = self
            .declaration_session_mut()
            .ok_or("Open declarations first.".to_owned())
            .and_then(|session| {
                session
                    .prepare_source(action)
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(work) => self.submit_tool(Work::Source(Box::new(work))),
            Err(error) => self.schema_feedback(error),
        }
    }
    pub(crate) fn bind_declaration_source(&mut self) {
        let path = self.workspaces.declarations.source.binding_path().trim();
        if !path.is_empty() {
            self.source_action(SourceAction::Bind(path.into()));
        }
    }
    pub(crate) fn import_declaration_draft(&mut self) {
        let path = self.workspaces.declarations.source.import_path().trim();
        if !path.is_empty() {
            self.source_action(SourceAction::ImportDraft(path.into()));
        }
    }
    pub(crate) fn cancel_declaration_action(&mut self) {
        if let Some(session) = self.declaration_session() {
            session.cancel();
            self.status = "Cancelling declaration action; waiting for its terminal receipt.".into();
        }
    }
    pub(crate) fn refresh_declaration_rows(&mut self) {
        self.source_action(SourceAction::Inspect);
    }
    pub(super) fn accept_source_tool(
        &mut self,
        prepared: &mut Prepared,
    ) -> Result<SourceRecovery, String> {
        let Prepared {
            receipt,
            preview,
            rows,
            validation,
        } = prepared;
        let archive = receipt.archive().map(PathBuf::from);
        let editable = receipt.editable_draft().map(PathBuf::from);
        self.workspaces
            .declarations
            .source
            .record_artifacts(archive.clone(), editable);
        if let Some(archive) = archive {
            self.schema_feedback(format!(
                "Previous declaration draft preserved at {}",
                archive.display()
            ));
        }
        let reloaded = receipt.reloads_output();
        let replaced = receipt.replaces_source();
        let recovery = self
            .declaration_session_mut()
            .ok_or("Declarations were closed.".to_owned())?
            .accept_source(receipt)
            .map_err(|error| error.to_string())?;
        if replaced {
            self.workspaces.declarations.source.retire_export();
        }
        self.workspaces
            .declarations
            .source
            .replace_preview(std::mem::take(preview), validation);
        if let Err(error) = validation {
            self.schema_feedback(format!("Declaration source needs repair:\n{error}"));
        }
        if let Some(rows) = rows.take() {
            self.workspaces.declarations.replace(rows);
        }
        if reloaded {
            self.workspaces.schema_refresh = Some(SchemaUpdate::Reloaded);
        }
        self.status = "Declaration source adopted; finishing recovery…".into();
        Ok(recovery)
    }
    pub(super) fn finish_source_tool(
        &mut self,
        receipt: &mut SourceRecovered,
    ) -> Result<(), String> {
        self.declaration_session_mut()
            .ok_or("Declarations were closed.".to_owned())?
            .accept_source_recovery(receipt)
            .map_err(|error| format!("Source buffer retained; recovery failed: {error}"))?;
        if self.workspaces.schema_refresh.is_some() {
            self.refresh_schema_context();
        } else {
            self.status = self.workspaces.declarations.source.status_message().into();
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests;
