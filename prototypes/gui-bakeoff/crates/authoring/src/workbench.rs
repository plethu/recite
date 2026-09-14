use crate::{Document, EditError, Passage, PassageKind, Preview, PreviewError, PreviewPage};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum View {
    Source,
    Passage(String),
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
            None => (View::Source, source.to_owned()),
        };
        Ok(Self {
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
    pub fn selected(&self) -> Result<Option<Passage>, WorkbenchError> {
        match &self.view {
            View::Source => Ok(None),
            View::Passage(id) => Ok(self.document.passages()?.into_iter().find(|p| &p.id == id)),
        }
    }
    pub fn select(&mut self, view: View) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        let text = match &view {
            View::Source => self.document.source().to_owned(),
            View::Passage(id) => {
                self.document
                    .passages()?
                    .into_iter()
                    .find(|p| &p.id == id)
                    .ok_or(EditError::MissingPassage)?
                    .text
            }
        };
        self.view = view;
        self.load(text);
        Ok(())
    }
    pub fn show_script(&mut self) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        let first = self
            .document
            .passages()?
            .into_iter()
            .next()
            .ok_or(EditError::MissingPassage)?;
        self.select(View::Passage(first.id))
    }
    pub fn apply(&mut self) -> Result<(), WorkbenchError> {
        match &self.view {
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
        let passage = self.selected()?.ok_or(EditError::MissingPassage)?;
        let id = self
            .document
            .add_choice(self.document.revision(), &passage.section)?;
        self.select(View::Passage(id))
    }
    pub fn start_preview(&mut self) -> Result<(), WorkbenchError> {
        self.require_applied()?;
        let selected = self.selected()?;
        let mut preview = Preview::at_block(
            &self.document,
            selected.as_ref().map(|passage| passage.section.as_str()),
        )?;
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
            View::Source => self.document.source().to_owned(),
            View::Passage(id) => match self
                .document
                .passages()
                .ok()
                .and_then(|passages| passages.into_iter().find(|p| &p.id == id))
            {
                Some(p) => p.text,
                None => {
                    self.view = View::Source;
                    self.document.source().to_owned()
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
