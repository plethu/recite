mod navigation;
mod structure;
use crate::{Document, EditError, PassageKind, Preview, PreviewError, PreviewPage};

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
    draft: String,
    loaded: String,
    draft_revision: i64,
    preview: Option<Preview>,
    page: Option<PreviewPage>,
    script_selection: Option<String>,
}

impl Workbench {
    pub fn new(source: &str) -> Result<Self, WorkbenchError> {
        Self::open(crate::DOCUMENT_NAME, source)
    }

    pub fn open(name: &str, source: &str) -> Result<Self, WorkbenchError> {
        let key = recite_core::DocumentKey::new(name).map_err(EditError::from)?;
        let document = Document::open(key, source)?;
        Self::from_document(document)
    }

    pub fn from_document(document: Document) -> Result<Self, WorkbenchError> {
        let source = document.source();
        let first = document
            .passages()
            .ok()
            .and_then(|passages| passages.into_iter().next());
        let revision = document.revision();
        let (view, draft) = match first {
            Some(first) => (View::Passage(first.id), first.text),
            None => match document
                .script()
                .ok()
                .and_then(|blocks| blocks.into_iter().next())
            {
                Some(block) => (View::Block(block.id), String::new()),
                None => (View::Source, source.to_owned()),
            },
        };
        let script_selection = document
            .script()
            .ok()
            .and_then(|blocks| blocks.first().map(|block| block.id.clone()));
        Ok(Self {
            script_selection,
            document,
            view,
            loaded: draft.clone(),
            draft,
            draft_revision: revision,
            preview: None,
            page: None,
        })
    }
    pub fn refresh_project(
        &mut self,
        context: crate::ProjectContext,
    ) -> Result<(), WorkbenchError> {
        self.document.refresh_project(context)?;
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
        &self.draft
    }
    pub fn set_draft(&mut self, draft: String) {
        self.draft = draft;
    }
    pub fn has_draft(&self) -> bool {
        self.draft != self.loaded
    }
    pub fn apply(&mut self) -> Result<(), WorkbenchError> {
        match &self.view {
            View::Block(_) => {}
            View::Source => self
                .document
                .replace_source(self.draft_revision, self.draft.clone())?,
            View::Passage(id) => {
                self.document
                    .replace_text(self.draft_revision, id, &self.draft)?
            }
        }
        self.loaded = self.draft.clone();
        self.draft_revision = self.document.revision();
        Ok(())
    }
    pub fn discard(&mut self) {
        self.draft = self.loaded.clone();
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
    pub fn start_preview(&mut self) -> Result<(), WorkbenchError> {
        self.start_preview_with(crate::PreviewSetup::default())
    }
    pub fn start_preview_with(&mut self, setup: crate::PreviewSetup) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        let selected = self.selected_block()?;
        let mut preview = Preview::configured(&self.document, selected.as_deref(), setup)?;
        let page = preview.advance(None)?;
        self.preview = Some(preview);
        self.page = Some(page);
        Ok(())
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
            View::Block(block) if self.document.sections().contains(block) => String::new(),
            View::Block(_) => return self.restore_script_selection(),
            View::Source => self.document.source().to_owned(),
            View::Passage(id) => match self
                .document
                .passages()
                .ok()
                .and_then(|passages| passages.into_iter().find(|p| &p.id == id))
            {
                Some(p) => p.text,
                None => {
                    return self.restore_script_selection();
                }
            },
        };
        self.load(next);
        Ok(())
    }
    fn load(&mut self, text: String) {
        self.draft = text.clone();
        self.loaded = text;
        self.draft_revision = self.document.revision();
    }
}
