//! Accepted Script projection and a paged, flat reading list; field drafts reuse it.
use recite_core::DocumentKey;
use recite_writer_model::{Passage, PassageKind, ScriptBlock, ScriptEntry, View, Workbench};
use std::sync::Arc;

pub(super) const PAGE: usize = 16;
pub(super) struct BeatPosition {
    index: usize,
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

#[derive(Clone, Copy)]
pub(super) struct PassageNumber(usize);
impl PassageNumber {
    pub fn ordinal(self) -> usize {
        self.0
    }
}

pub(super) enum Content {
    Passage {
        passage: Passage,
        number: PassageNumber,
    },
    Heading(String),
    Note(String),
    Continuation(String),
}

pub(super) struct Row {
    pub content: Content,
    pub groups: Arc<[String]>,
    pub reply: Option<usize>,
}

pub(super) struct Reading {
    key: Option<DocumentKey>,
    source: Option<Arc<str>>,
    script: Result<Arc<[ScriptBlock]>, String>,
    block: Option<String>,
    view: Option<View>,
    pub selected: Option<String>,
    pub rows: Arc<[Row]>,
    pub selected_row: Option<usize>,
    pub replies: usize,
    pub is_default: bool,
    pub position: Option<BeatPosition>,
    pub error: Option<String>,
    pub page: usize,
}

impl Reading {
    pub fn selected_passage(&self) -> Option<&Passage> {
        match &self.rows.get(self.selected_row?)?.content {
            Content::Passage { passage, .. } => Some(passage),
            _ => None,
        }
    }
    pub fn new() -> Self {
        Self {
            key: None,
            source: None,
            script: Ok(Arc::from([])),
            block: None,
            view: None,
            selected: None,
            rows: Arc::from([]),
            selected_row: None,
            replies: 0,
            is_default: false,
            position: None,
            error: None,
            page: 0,
        }
    }

    /// Called at file/edit/navigation boundaries, never by a renderer.
    pub fn prepare(&mut self, model: &Workbench) {
        let source = model.document().source_snapshot();
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
            self.source = Some(source);
            self.block = None;
            self.rows = Arc::from([]);
            self.error = None;
            self.page = 0;
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
            self.page = 0;
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
        }
        self.selected_row = match model.view() {
            View::Passage(id) => self.rows.iter().position(
                |row| matches!(&row.content, Content::Passage { passage, .. } if &passage.id == id),
            ),
            View::Block(_) | View::Source => None,
        };
        if let Some(index) = self.selected_row {
            self.page = index / PAGE;
        }
    }

    pub fn script(&self) -> Result<&[ScriptBlock], &str> {
        self.script.as_deref().map_err(String::as_str)
    }
}

pub(crate) fn flatten(entries: &[ScriptEntry]) -> Arc<[Row]> {
    let mut rows = Vec::new();
    let mut replies = 0;
    let mut passages = 0;
    let mut pending = vec![(entries.iter(), Arc::<[String]>::from([]))];
    while let Some((items, groups)) = pending.last_mut() {
        let Some(entry) = items.next() else {
            pending.pop();
            continue;
        };
        let groups = groups.clone();
        let content = match entry {
            ScriptEntry::Passage(passage) => {
                passages += 1;
                Content::Passage {
                    passage: passage.clone(),
                    number: PassageNumber(passages),
                }
            }
            ScriptEntry::Group { heading, entries } => {
                let mut nested = groups.to_vec();
                nested.push(heading.clone());
                pending.push((entries.iter(), nested.into()));
                Content::Heading(heading.clone())
            }
            ScriptEntry::Jump(target) => Content::Continuation(target.clone()),
            ScriptEntry::Effect(text) => Content::Note(format!("Effect · {text}")),
            ScriptEntry::Source(text) => {
                Content::Note(format!("Source fragment (read only) · {text}"))
            }
        };
        let reply = if matches!(&content, Content::Passage { passage, .. } if matches!(passage.kind, PassageKind::Choice { .. }))
        {
            replies += 1;
            Some(replies)
        } else {
            None
        };
        rows.push(Row {
            content,
            groups,
            reply,
        });
    }
    rows.into()
}

#[cfg(test)]
mod tests;
