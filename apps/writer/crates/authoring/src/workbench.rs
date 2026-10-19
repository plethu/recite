mod edit;
mod preparing;
pub use preparing::{NavigationRequest, PreparedNavigation};
mod navigation;
mod opening;
mod preview;
pub use edit::{EditRequest, PreparedEdit};
mod structure;
use crate::{Document, DraftSnapshot, EditError, PassageKind, Preview, PreviewError, PreviewPage};
use recite_compiler::authoring::{CancellationToken, WorkControl};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum View {
    Source,
    Passage(String),
    Block(String),
}

#[derive(Debug, thiserror::Error)]
pub enum WorkbenchError {
    #[error(transparent)]
    Edit(#[from] EditError),
    #[error(transparent)]
    Preview(Box<PreviewError>),
    #[error("Apply or discard the field draft before changing context.")]
    DraftPending,
    #[error("Start a preview first.")]
    NoPreview,
    #[error("The bundled writer example schema is invalid.")]
    ExampleSchema,
}

impl From<PreviewError> for WorkbenchError {
    fn from(error: PreviewError) -> Self {
        Self::Preview(Box::new(error))
    }
}

/// Shared interaction state; toolkit adapters own only widget presentation.
pub struct Workbench {
    document: Document,
    view: View,
    draft: DraftSnapshot,
    loaded: DraftSnapshot,
    dirty: bool,
    draft_revision: i64,
    preview: Option<Preview>,
    page: Option<PreviewPage>,
    script_selection: Option<String>,
}

impl Workbench {
    pub fn refresh_project(
        &mut self,
        context: crate::ProjectContext,
    ) -> Result<(), WorkbenchError> {
        self.refresh_project_with_control(context, &CancellationToken::new())
    }

    /// Preserve drafts if cooperative project analysis is interrupted.
    pub fn refresh_project_with_control(
        &mut self,
        context: crate::ProjectContext,
        control: &dyn WorkControl,
    ) -> Result<(), WorkbenchError> {
        self.document
            .refresh_project_with_control(context, control)?;
        self.draft_revision = self.document.revision();
        Ok(())
    }

    pub const fn document(&self) -> &Document {
        &self.document
    }
    pub const fn view(&self) -> &View {
        &self.view
    }
    pub fn draft(&self) -> &str {
        self.draft.as_str()
    }
    pub fn set_draft(&mut self, draft: String) {
        // Move the widget's buffer; snapshots share it without copying prose.
        self.set_draft_snapshot(draft.into());
    }
    /// Replace the field draft without applying it to the accepted document.
    pub fn set_draft_snapshot(&mut self, draft: DraftSnapshot) {
        self.dirty = draft != self.loaded;
        self.draft = draft;
    }
    /// Borrow the immutable draft without requiring contiguous source text.
    pub const fn draft_snapshot(&self) -> &DraftSnapshot {
        &self.draft
    }
    pub const fn has_draft(&self) -> bool {
        self.dirty
    }
    pub fn apply(&mut self) -> Result<(), WorkbenchError> {
        if !self.dirty && self.view != View::Source {
            return Ok(());
        }
        match &self.view {
            View::Block(_) => {}
            View::Source => self
                .document
                .replace_source(self.draft_revision, self.draft.text_snapshot())?,
            View::Passage(id) => {
                self.document
                    .replace_text(self.draft_revision, id, self.draft.as_str())?
            }
        }
        if self.view == View::Source {
            // Acceptance shares these bytes now; release the Rope draft owner.
            self.draft = self.document.source_snapshot().into();
        }
        self.loaded = self.draft.clone();
        self.dirty = false;
        self.draft_revision = self.document.revision();
        Ok(())
    }
    pub fn discard(&mut self) {
        self.draft = self.loaded.clone();
        self.dirty = false;
    }
    pub fn undo(&mut self) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        self.document.undo()?;
        self.refresh()
    }
    pub fn redo(&mut self) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        self.document.redo()?;
        self.refresh()
    }
    pub fn attribute(&mut self, value: &str) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        let passage = self.selected()?.ok_or(EditError::MissingPassage)?;
        match passage.kind {
            PassageKind::Dialogue { .. } => {
                self.document
                    .set_speaker(self.document.revision(), &passage.id, value)?
            }
            PassageKind::Choice { .. } => {
                self.document
                    .set_destination(self.document.revision(), &passage.id, value)?
            }
        }
        self.refresh()
    }
    pub fn add_choice(&mut self) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        let block = self.selected_block()?.ok_or(EditError::Destination)?;
        let id = self.document.add_choice(self.document.revision(), &block)?;
        self.select(View::Passage(id))
    }
    pub fn advance_preview(&mut self, choice_index: Option<usize>) -> Result<(), WorkbenchError> {
        let choice = choice_index
            .map(|index| {
                self.page
                    .as_ref()
                    .and_then(|p| p.choices.get(index))
                    .map(|c| c.id.clone())
                    .ok_or(WorkbenchError::NoPreview)
            })
            .transpose()?;
        let preview = self.preview.as_mut().ok_or(WorkbenchError::NoPreview)?;
        self.page = Some(preview.advance(choice)?);
        Ok(())
    }
    pub fn answer_preview(
        &mut self,
        value: recite_runtime::ConditionValue,
    ) -> Result<(), WorkbenchError> {
        let request = self
            .page
            .as_ref()
            .and_then(|p| p.condition.clone())
            .ok_or(WorkbenchError::NoPreview)?;
        let preview = self.preview.as_mut().ok_or(WorkbenchError::NoPreview)?;
        self.page = Some(preview.answer(&request, value)?);
        Ok(())
    }
    pub fn acknowledge_preview(
        &mut self,
        ack: recite_runtime::EffectAck,
    ) -> Result<(), WorkbenchError> {
        let id = self
            .page
            .as_ref()
            .and_then(|p| p.waiting_effect.clone())
            .ok_or(WorkbenchError::NoPreview)?;
        let preview = self.preview.as_mut().ok_or(WorkbenchError::NoPreview)?;
        self.page = Some(preview.acknowledge(id, ack)?);
        Ok(())
    }
    pub fn preview_trace(&self) -> Option<&recite_runtime::preview::PreviewTrace> {
        self.preview.as_ref().map(Preview::trace)
    }

    pub fn preview_events(&self) -> &[recite_runtime::preview::PreviewEvent] {
        self.preview.as_ref().map_or(&[], Preview::events)
    }

    pub fn preview_page(&self) -> Option<&PreviewPage> {
        self.page.as_ref()
    }
    pub fn preview_stale(&self) -> bool {
        self.has_draft()
            || self
                .preview
                .as_ref()
                .is_some_and(|p| p.revision() != self.document.revision())
    }
    fn require_applied(&self) -> Result<(), WorkbenchError> {
        if self.has_draft() {
            return Err(WorkbenchError::DraftPending);
        }
        Ok(())
    }
    fn refresh(&mut self) -> Result<(), WorkbenchError> {
        let next = match &self.view {
            View::Block(block) if self.document.sections().contains(block) => String::new().into(),
            View::Block(_) => return self.restore_script_selection(),
            View::Source => self.document.source_snapshot().into(),
            View::Passage(id) => match self.document.passage_snapshot().ok().and_then(|passages| {
                passages
                    .iter()
                    .find(|p| &p.id == id)
                    .map(|p| p.text.clone())
            }) {
                Some(text) => text.into(),
                None => {
                    return self.restore_script_selection();
                }
            },
        };
        self.load(next);
        Ok(())
    }
    fn load(&mut self, text: DraftSnapshot) {
        self.draft = text;
        self.loaded = self.draft.clone();
        self.dirty = false;
        self.draft_revision = self.document.revision();
    }
}
