//! Accepted Script projection and bounded overlapping reading; field drafts reuse it.
use recite_core::DocumentKey;
use recite_writer_model::{Passage, PassageKind, ScriptBlock, View, Workbench};
use std::sync::Arc;

mod anchor;
pub use anchor::Anchor;
pub mod galley;
pub mod navigation;
mod rows;
pub mod surface;
pub(crate) use rows::flatten;
pub use rows::{Content, PassageNumber, Row};
pub(crate) mod window;
pub use window::CAPACITY as PAGE;
pub use window::{Direction, ReadingWindow};
pub struct BeatPosition {
    pub(crate) index: usize,
    total: usize,
}
impl BeatPosition {
    pub fn ordinal(&self) -> usize {
        self.index + 1
    }
    pub fn total(&self) -> usize {
        self.total
    }
}

pub struct Reading {
    key: Option<DocumentKey>,
    pub source: Option<Arc<str>>,
    script: Result<Arc<[ScriptBlock]>, String>,
    /// Every beat's rows, in scene order; see `galley`.
    beats: Arc<[Arc<[Row]>]>,
    block: Option<String>,
    view: Option<View>,
    pub selected: Option<String>,
    pub rows: Arc<[Row]>,
    pub selected_row: Option<usize>,
    pub replies: usize,
    pub is_default: bool,
    pub position: Option<BeatPosition>,
    pub error: Option<String>,
    pub window: ReadingWindow,
}

impl Reading {
    pub fn selected_passage(&self) -> Option<&Passage> {
        match &self.rows.get(self.selected_row?)?.content {
            Content::Passage { passage, .. } => Some(passage),
            _ => None,
        }
    }
    /// The passage `id` among the selected beat's rows.
    pub fn passage(&self, id: &str) -> Option<&Passage> {
        self.rows.iter().find_map(|row| match &row.content {
            Content::Passage { passage, .. } if passage.id == id => Some(passage),
            _ => None,
        })
    }
    /// Where `owner` leads now: a reply's destination, or the selected beat's continuation.
    pub fn destination(&self, owner: &recite_writer_model::RouteOwner) -> Option<&str> {
        match owner {
            recite_writer_model::RouteOwner::Reply(id) => match &self.passage(id)?.kind {
                PassageKind::Choice { destination } => destination.as_deref(),
                PassageKind::Dialogue { .. } => None,
            },
            recite_writer_model::RouteOwner::Beat(id) if self.selected.as_ref() == Some(id) => {
                self.rows.iter().find_map(|row| match &row.content {
                    Content::Continuation(target) if row.groups.is_empty() => Some(target.as_str()),
                    _ => None,
                })
            }
            _ => None,
        }
    }
    pub fn new() -> Self {
        Self {
            key: None,
            source: None,
            script: Ok(Arc::from([])),
            beats: Arc::from([]),
            block: None,
            view: None,
            selected: None,
            rows: Arc::from([]),
            selected_row: None,
            replies: 0,
            is_default: false,
            position: None,
            error: None,
            window: ReadingWindow::default(),
        }
    }

    /// Called at file/edit/navigation boundaries, never by a renderer.
    pub fn prepare(&mut self, model: &Workbench) {
        let source = model.document().source_snapshot();
        let changed_document = self.key.as_ref() != Some(model.document().key());
        let previous_beat = self.selected.clone();
        if self.key.as_ref() != Some(model.document().key())
            || self
                .source
                .as_ref()
                .is_none_or(|old| !Arc::ptr_eq(old, &source))
        {
            self.key = Some(model.document().key().clone());
            self.script = model
                .document()
                .script_snapshot()
                .map_err(|error| crate::feedback::edit(&error));
            self.shape_beats();
            self.source = Some(source);
            self.block = None;
            self.rows = Arc::from([]);
            self.error = None;
            if changed_document {
                self.window = ReadingWindow::default();
            }
            self.view = None;
            self.position = None;
            self.is_default = false;
        }
        if self.view.as_ref() == Some(model.view()) {
            return;
        }
        self.view = Some(model.view().clone());
        self.selected = model.selected_block().ok().flatten();
        self.error = self.selected.as_deref().and_then(|id| {
            model
                .document()
                .validate_script_block(id)
                .err()
                .map(|error| crate::feedback::edit(&error))
        });
        let Some(id) = self.selected.as_ref() else {
            self.selected_row = None;
            return;
        };
        if self.block.as_ref() != Some(id) {
            self.block = Some(id.clone());
            if previous_beat.as_ref() != Some(id) {
                self.window = ReadingWindow::default();
            }
            self.is_default = self
                .script
                .as_ref()
                .ok()
                .and_then(|blocks| blocks.iter().find(|block| &block.id == id))
                .is_some_and(|block| block.is_default);
            self.position = self.script.as_ref().ok().and_then(|blocks| {
                blocks
                    .iter()
                    .position(|block| &block.id == id)
                    .map(|index| BeatPosition {
                        index,
                        total: blocks.len(),
                    })
            });
            self.rows = if self.error.is_none() {
                self.script
                    .as_ref()
                    .ok()
                    .and_then(|blocks| blocks.iter().find(|block| &block.id == id))
                    .map(|block| flatten(&block.entries))
                    .unwrap_or_default()
            } else {
                Arc::from([])
            };
            self.replies = self.rows.iter().filter(|row| row.reply.is_some()).count();
            self.window.normalize(self.rows.len());
        }
        self.selected_row = match model.view() {
            View::Passage(id) => self.rows.iter().position(
                |row| matches!(&row.content, Content::Passage { passage, .. } if &passage.id == id),
            ),
            View::Block(_) | View::Source => None,
        };
        self.reveal_selected();
    }

    pub(crate) fn reveal_selected(&mut self) {
        if let Some(index) = self.selected_row {
            self.window.reveal(index);
        }
    }

    pub fn script(&self) -> Result<&[ScriptBlock], &str> {
        self.script.as_deref().map_err(String::as_str)
    }
}

impl Default for Reading {
    fn default() -> Self {
        Self::new()
    }
}
