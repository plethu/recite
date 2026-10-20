//! Accepted Script projection and a paged, flat reading list; field drafts reuse it.
use recite_core::DocumentKey;
use recite_writer_model::{Passage, ScriptBlock, ScriptEntry, View, Workbench};
use std::sync::Arc;

pub(super) enum Content {
    Passage(Passage),
    Heading(String),
    Note(String),
    Continuation(String),
}

pub(super) struct Row {
    pub content: Content,
    pub groups: Arc<[String]>,
}

pub(super) struct Reading {
    key: Option<DocumentKey>,
    source: Option<Arc<str>>,
    script: Result<Arc<[ScriptBlock]>, String>,
    block: Option<String>,
    view: Option<View>,
    pub selected: Option<String>,
    pub rows: Arc<[Row]>,
    pub error: Option<String>,
    pub page: usize,
}

impl Reading {
    pub fn new() -> Self {
        Self {
            key: None,
            source: None,
            script: Ok(Arc::from([])),
            block: None,
            view: None,
            selected: None,
            rows: Arc::from([]),
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
        let View::Block(id) = model.view() else {
            return;
        };
        if self.block.as_ref() == Some(id) {
            return;
        }
        self.block = Some(id.clone());
        self.page = 0;
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
    }

    pub fn script(&self) -> Result<&[ScriptBlock], &str> {
        self.script.as_deref().map_err(String::as_str)
    }
}

fn flatten(entries: &[ScriptEntry]) -> Arc<[Row]> {
    let mut rows = Vec::new();
    let mut pending = vec![(entries.iter(), Arc::<[String]>::from([]))];
    while let Some((items, groups)) = pending.last_mut() {
        let Some(entry) = items.next() else {
            pending.pop();
            continue;
        };
        let groups = groups.clone();
        let content = match entry {
            ScriptEntry::Passage(passage) => Content::Passage(passage.clone()),
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
        rows.push(Row { content, groups });
    }
    rows.into()
}

#[cfg(test)]
mod tests;
