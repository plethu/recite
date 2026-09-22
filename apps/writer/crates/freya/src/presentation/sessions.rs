//! Per-document presentation is transient; authored drafts belong to the buffers.
use crate::editing::{Pane, Writer};
use freya::prelude::*;
use recite_config::WriterView;
use std::collections::BTreeMap;

#[derive(Clone)]
struct Position {
    view: WriterView,
    pane: Pane,
    selected: Option<String>,
    reading: (i32, i32),
    source: (i32, i32),
}
#[derive(Clone, Copy)]
pub(super) struct Sessions(State<BTreeMap<String, Position>>);
impl Sessions {
    pub fn new() -> Self {
        Self(use_state(BTreeMap::new))
    }
    pub fn remember(mut self, writer: Writer) {
        let Some(key) = key(writer) else {
            return;
        };
        self.0.write().insert(
            key,
            Position {
                view: *writer.layout.view.peek(),
                pane: *writer.pane.peek(),
                selected: writer.selection.peek().clone(),
                reading: writer.scroll.into(),
                source: writer.source_viewport.scroll.into(),
            },
        );
    }
    pub fn restore(self, mut writer: Writer) -> bool {
        let position = key(writer).and_then(|key| self.0.peek().get(&key).cloned());
        let Some(position) = position else {
            return false;
        };
        writer.layout.view.set(position.view);
        writer.pane.set(position.pane);
        writer.selection.set(position.selected);
        self.restore_scroll(writer);
        true
    }
    pub fn restore_scroll(self, mut writer: Writer) {
        let position = key(writer).and_then(|key| self.0.peek().get(&key).cloned());
        if let Some(position) = position {
            writer.scroll.scroll_to_x(position.reading.0);
            writer.scroll.scroll_to_y(position.reading.1);
            writer.source_viewport.scroll.scroll_to_x(position.source.0);
            writer.source_viewport.scroll.scroll_to_y(position.source.1);
        }
    }
}
fn key(writer: Writer) -> Option<String> {
    if let Some(files) = writer.files.peek().as_ref() {
        Some(format!("file:{}", files.current.display()))
    } else {
        writer
            .buffers
            .model
            .peek()
            .as_ref()
            .ok()
            .map(|m| format!("example:{}", m.document().key()))
    }
}
impl Writer {
    pub fn remember_scene(self) {
        self.layout.sessions.remember(self);
    }
    pub fn resume_scene(self) -> Result<(), String> {
        if self.layout.sessions.restore(self) {
            Ok(())
        } else {
            self.scene_opened()
        }
    }
    pub fn restore_scroll(self) {
        self.layout.sessions.restore_scroll(self);
    }
    pub fn open_document(self, path: &std::path::Path) -> Result<(), String> {
        self.remember_scene();
        self.buffers
            .switch(self.files, path, self.dark, |_| Ok(()))?;
        self.resume_scene()
    }
}
