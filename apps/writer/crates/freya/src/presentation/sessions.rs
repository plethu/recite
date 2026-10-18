//! Per-document presentation is transient; authored drafts belong to the buffers.
use crate::editing::{Pane, Writer};
use freya::prelude::*;
use recite_config::WriterView;
use std::collections::BTreeMap;

struct Position {
    view: WriterView,
    pane: Pane,
    selected: Option<String>,
    reading: (i32, i32),
    source: (i32, i32),
    rules: Option<recite_writer_model::ReplyRules>,
}
pub(crate) struct RetiredPresentation {
    positions: BTreeMap<String, Position>,
    rules: Option<recite_writer_model::ReplyRules>,
}
impl RetiredPresentation {
    fn is_empty(&self) -> bool {
        self.positions.is_empty() && self.rules.is_none()
    }
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
        let rules = self.0.write().get_mut(&key).and_then(|p| p.rules.take());
        self.0.write().insert(
            key,
            Position {
                view: *writer.layout.view.peek(),
                pane: *writer.pane.peek(),
                selected: writer.selection.peek().clone(),
                reading: writer.scroll.into(),
                source: writer.source_viewport.scroll.into(),
                rules,
            },
        );
    }
    pub fn scene_key(self, writer: Writer) -> Option<String> {
        key(writer)
    }
    pub fn switch_rules(
        mut self,
        previous: Option<String>,
        next: Option<String>,
        mut rules: State<Option<recite_writer_model::ReplyRules>>,
    ) {
        let outgoing = rules.write().take();
        let mut sessions = self.0.write();
        if let Some(previous) = previous.and_then(|key| sessions.get_mut(&key)) {
            previous.rules = outgoing;
        }
        let incoming = next
            .and_then(|key| sessions.get_mut(&key))
            .and_then(|p| p.rules.take());
        rules.set(incoming);
    }
    pub fn close(mut self, path: &std::path::Path) -> RetiredPresentation {
        // Keep cheap orientation for reopening; closed tabs release authored Rules.
        let rules = self
            .0
            .write()
            .get_mut(&file_key(path))
            .and_then(|position| {
                if position.pane == Pane::Rules {
                    position.pane = Pane::Map;
                }
                position.rules.take()
            });
        RetiredPresentation {
            positions: BTreeMap::new(),
            rules,
        }
    }
    pub fn reset(
        mut self,
        mut rules: State<Option<recite_writer_model::ReplyRules>>,
    ) -> RetiredPresentation {
        RetiredPresentation {
            positions: std::mem::take(&mut *self.0.write()),
            rules: rules.write().take(),
        }
    }
    pub fn has_file(self, path: &std::path::Path) -> bool {
        self.0.peek().contains_key(&file_key(path))
    }
    pub fn view_for_file(self, path: &std::path::Path, fallback: WriterView) -> WriterView {
        self.0
            .peek()
            .get(&file_key(path))
            .map_or(fallback, |position| position.view)
    }
    pub fn restore(self, mut writer: Writer) -> bool {
        let position = key(writer).and_then(|key| {
            self.0
                .peek()
                .get(&key)
                .map(|p| (p.view, p.pane, p.selected.clone()))
        });
        let Some((view, pane, selected)) = position else {
            return false;
        };
        writer.layout.view.set(view);
        writer.pane.set(pane);
        writer.selection.set(selected);
        self.restore_scroll(writer);
        true
    }
    pub fn restore_scroll(self, mut writer: Writer) {
        let position =
            key(writer).and_then(|key| self.0.peek().get(&key).map(|p| (p.reading, p.source)));
        if let Some((reading, source)) = position {
            writer.scroll.scroll_to_x(reading.0);
            writer.scroll.scroll_to_y(reading.1);
            writer.source_viewport.scroll.scroll_to_x(source.0);
            writer.source_viewport.scroll.scroll_to_y(source.1);
        }
    }
}
fn file_key(path: &std::path::Path) -> String {
    format!("file:{}", path.display())
}
fn key(writer: Writer) -> Option<String> {
    if let Some(files) = writer.files.peek().as_ref() {
        Some(file_key(&files.current))
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
    pub(crate) fn scene_key(self) -> Option<String> {
        self.layout.sessions.scene_key(self)
    }
    pub(crate) fn switch_rules(self, previous: Option<String>) {
        self.layout
            .sessions
            .switch_rules(previous, self.scene_key(), self.buffers.rules);
    }
    pub(crate) fn close_scene(self, path: &std::path::Path) {
        self.retire_presentation(self.layout.sessions.close(path));
    }
    pub(crate) fn reset_scenes(self) {
        self.retire_presentation(self.layout.sessions.reset(self.buffers.rules));
    }
    fn retire_presentation(mut self, retired: RetiredPresentation) {
        if !retired.is_empty()
            && let Err(error) = crate::editor_work::worker::Worker::retire_presentation(retired)
        {
            self.message.error(error.to_string());
        }
    }
    pub fn remember_scene(self) {
        self.layout.sessions.remember(self);
    }
    pub fn resume_scene(self) -> Result<(), String> {
        if self.layout.sessions.restore(self) {
            Ok(())
        } else {
            self.scene_opened(self.preferences.peek().config.writer.view)
        }
    }
    pub fn restore_scroll(self) {
        self.layout.sessions.restore_scroll(self);
    }
    pub fn resume_prepared_scene(mut self, view: WriterView) {
        if self.layout.sessions.restore(self) {
            return;
        }
        let source = self
            .buffers
            .model
            .peek()
            .as_ref()
            .is_ok_and(|m| m.view() == &recite_writer_model::View::Source);
        self.selection.set(None);
        self.layout
            .view
            .set(if source { WriterView::Source } else { view });
        self.pane.set(
            if !source && (self.layout.standalone() || *self.pane.peek() == Pane::Script) {
                Pane::Script
            } else {
                Pane::Map
            },
        );
    }
    pub fn open_document(
        self,
        path: &std::path::Path,
        focus: Option<AccessibilityId>,
    ) -> Result<(), String> {
        let mut message = self.message;
        self.open_document_then(
            path,
            EventHandler::new(move |result: Result<(), String>| {
                if result.is_ok()
                    && let Some(focus) = focus
                {
                    focus.request_focus();
                }
                message.report(result, "Scene opened.".into());
            }),
        )
    }
    pub(crate) fn open_document_then(
        self,
        path: &std::path::Path,
        completed: EventHandler<Result<(), String>>,
    ) -> Result<(), String> {
        self.work.require_idle()?;
        if self
            .files
            .peek()
            .as_ref()
            .is_some_and(|p| p.current == path)
        {
            completed.call(Ok(()));
            return Ok(());
        }
        self.remember_scene();
        self.buffers.harvest();
        let view = self
            .layout
            .sessions
            .view_for_file(path, self.preferences.peek().config.writer.view);
        crate::editor_work::navigate(
            self,
            crate::navigation::request::Request {
                target: crate::navigation::request::Target::Document {
                    path: path.to_owned(),
                    view,
                    initial: !self.layout.sessions.has_file(path),
                },
                cancelled: None,
            },
            completed,
        )
    }
}

#[cfg(test)]
mod tests;
