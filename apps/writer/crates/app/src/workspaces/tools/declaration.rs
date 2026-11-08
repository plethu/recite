//! Checked source adoption and its recovery fence share one worker continuation.
pub use super::super::{ContextUpdate, declarations::producer_description, text};
use super::*;
use crate::external_editor::Request as EditorRequest;
use recite_writer_session::declarations::SourceStatus;
pub struct Prepared {
    receipt: DeclarationReceipt,
    rows: Option<Vec<Row>>,
    presentation: Presentation,
}
pub enum Presentation {
    Source {
        preview: Vec<String>,
        validation: Result<SourceStatus, String>,
    },
    Registration(Vec<String>),
    Output,
}
#[derive(Clone, Copy)]
pub enum Completion {
    Source,
    Registration,
    Output,
}
impl Prepared {
    pub fn new(receipt: DeclarationReceipt) -> Self {
        let presentation = if let Some(registration) = receipt.registration() {
            Presentation::Registration(producer_description(registration))
        } else if receipt.inspects_source() {
            Presentation::Source {
                preview: receipt.draft().map(text::pages).unwrap_or_default(),
                validation: receipt.inspect_source().map_err(|error| error.to_string()),
            }
        } else {
            Presentation::Output
        };
        let rows = receipt
            .last_valid_schema()
            .map(|schema| entries::entries(schema).into_iter().map(Row::new).collect());
        Self {
            receipt,
            rows,
            presentation,
        }
    }
    pub(super) fn completion(&self) -> Completion {
        match self.presentation {
            Presentation::Source { .. } => Completion::Source,
            Presentation::Registration(_) => Completion::Registration,
            Presentation::Output => Completion::Output,
        }
    }
}
impl Writer {
    pub fn open_declaration_source(&mut self) {
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
    pub fn open_declaration_draft(&mut self) {
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
    pub fn declaration_action(&mut self, action: DeclarationAction) {
        if self.declaration_action_blocked() {
            return;
        }
        self.workspaces.schema_feedback.clear();
        self.workspaces.schema_feedback_page = 0;
        if matches!(&action, DeclarationAction::ReloadGenerated) {
            self.workspaces.declarations.source.invalidate_output();
        }
        let result = self
            .declaration_session_mut()
            .ok_or("Open declarations first.".to_owned())
            .and_then(|session| {
                session
                    .prepare_declaration(action)
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(work) => self.submit_tool(Work::Declaration(Box::new(work))),
            Err(error) => self.schema_feedback(error),
        }
    }
    pub fn bind_declaration_source(&mut self) {
        let path = self.workspaces.declarations.source.binding_path().trim();
        if !path.is_empty() {
            self.declaration_action(DeclarationAction::Bind(path.into()));
        }
    }
    pub fn import_declaration_draft(&mut self) {
        let path = self.workspaces.declarations.source.import_path().trim();
        if !path.is_empty() {
            self.declaration_action(DeclarationAction::ImportDraft(path.into()));
        }
    }
    pub fn cancel_declaration_action(&mut self) {
        if let Some(session) = self.declaration_session() {
            session.cancel();
            self.status = "Cancelling declaration action; waiting for its terminal receipt.".into();
        }
    }
    pub fn refresh_declaration_rows(&mut self) {
        self.declaration_action(DeclarationAction::InspectSource);
    }
    pub(super) fn accept_declaration_tool(
        &mut self,
        prepared: &mut Prepared,
    ) -> Result<DeclarationFinish, String> {
        let Prepared {
            receipt,
            rows,
            presentation,
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
            .accept_declaration(receipt)
            .map_err(|error| error.to_string())?;
        if replaced {
            self.workspaces.declarations.source.retire_export();
        }
        match presentation {
            Presentation::Source {
                preview,
                validation,
            } => {
                self.workspaces
                    .declarations
                    .source
                    .replace_preview(std::mem::take(preview), validation);
                if let Err(error) = validation {
                    self.schema_feedback(format!("Declaration source needs repair:\n{error}"));
                }
            }
            Presentation::Registration(pages) => self
                .workspaces
                .declarations
                .describe_producer(std::mem::take(pages)),
            Presentation::Output => (),
        }
        if let Some(rows) = rows.take() {
            self.workspaces.declarations.replace(rows);
        }
        if reloaded {
            self.workspaces.context_refresh = Some(ContextUpdate::Reloaded);
            if !receipt.inspects_source() {
                self.workspaces.declarations.source.invalidate_output();
            }
        }
        self.status = "Declaration action adopted; finishing recovery and cleanup…".into();
        Ok(recovery)
    }
    pub(super) fn finish_declaration_tool(
        &mut self,
        receipt: &mut DeclarationFinished,
        completion: Completion,
    ) -> Result<(), String> {
        self.declaration_session_mut()
            .ok_or("Declarations were closed.".to_owned())?
            .accept_declaration_finish(receipt)
            .map_err(|error| format!("Source buffer retained; recovery failed: {error}"))?;
        if self.workspaces.context_refresh.is_some() {
            self.refresh_project_context();
        } else {
            match completion {
                Completion::Source => {
                    self.status = self.workspaces.declarations.source.status_message().into()
                }
                Completion::Registration => match self
                    .declaration_session()
                    .map(|session| &session.registration)
                {
                    Some(Err(error)) => {
                        self.schema_feedback(format!("Producer registration is invalid:\n{error}"))
                    }
                    _ => self.status =
                        "Producer registration reloaded. Review the command before regenerating."
                            .into(),
                },
                Completion::Output => self.status = "Generated schema reloaded.".into(),
            }
        }
        Ok(())
    }
}
