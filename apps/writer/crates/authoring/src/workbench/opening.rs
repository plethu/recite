//! Choose the initial field without projecting Script for a Source editor.
use super::{View, Workbench, WorkbenchError};
use crate::{Document, DraftSnapshot, EditError};

impl Workbench {
    pub fn new(source: &str) -> Result<Self, WorkbenchError> {
        Self::open(crate::DOCUMENT_NAME, source)
    }

    pub fn open(name: &str, source: &str) -> Result<Self, WorkbenchError> {
        let key = recite_core::DocumentKey::new(name).map_err(EditError::from)?;
        let document = Document::open(key, source)?;
        Self::from_document(document, None)
    }

    pub fn from_document(
        document: Document,
        initial: Option<View>,
    ) -> Result<Self, WorkbenchError> {
        let revision = document.revision();
        let (view, draft, script_selection) = if initial.as_ref() == Some(&View::Source) {
            (View::Source, document.source_snapshot().into(), None)
        } else {
            // Script construction also fills the passage cache from its parse.
            let script = document.script_snapshot().ok();
            let passages = document.passage_snapshot().ok();
            let first = passages.as_ref().and_then(|passages| passages.first());
            let first_block = script.as_ref().and_then(|blocks| blocks.first());
            let (view, draft) = match first {
                Some(first) => (
                    View::Passage(first.id.clone()),
                    DraftSnapshot::from(first.text.clone()),
                ),
                None => match first_block {
                    Some(block) => (
                        View::Block(block.id.clone()),
                        DraftSnapshot::from(String::new()),
                    ),
                    None => (
                        View::Source,
                        DraftSnapshot::from(document.source_snapshot()),
                    ),
                },
            };
            let script_selection = first_block.map(|block| block.id.clone());
            (view, draft, script_selection)
        };
        let mut workbench = Self {
            script_selection,
            document,
            view,
            loaded: draft.clone(),
            draft,
            dirty: false,
            draft_revision: revision,
            preview: None,
            page: None,
        };
        if let Some(view) = initial {
            workbench.select(view)?;
        }
        Ok(workbench)
    }
}
