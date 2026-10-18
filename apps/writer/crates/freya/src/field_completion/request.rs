//! Completion spans belong to one analysis, Rope and cursor.
use crate::editing::Writer;
use freya::{code_editor::Rope, text_edit::TextEditor};
use recite_core::SourcePosition;
use recite_writer_model::{CompletionSnapshot, SourceCompletions, View};
use std::sync::Arc;

pub(super) struct Request {
    pub analysis: CompletionSnapshot,
    pub draft: Rope,
    pub position: SourcePosition,
}
impl Request {
    pub fn capture(writer: Writer) -> Option<Self> {
        let editor = writer.buffers.editor.read();
        let offset = editor
            .rope
            .utf16_cu_to_char(editor.cursor_pos().min(editor.rope.len_utf16_cu()));
        let line = editor.rope.char_to_line(offset);
        let column = offset - editor.rope.line_to_char(line);
        let position = SourcePosition::new(
            u32::try_from(line + 1).ok()?,
            u32::try_from(column + 1).ok()?,
        )
        .ok()?;
        let model = writer.buffers.model.read();
        let model = model.as_ref().ok()?;
        if model.view() != &View::Source {
            return None;
        }
        Some(Self {
            analysis: model.document().completion_snapshot(),
            draft: editor.rope.clone(),
            position,
        })
    }
    pub fn matches(&self, other: &Self) -> bool {
        self.analysis.same_analysis(&other.analysis)
            && self.position == other.position
            && (self.draft.is_instance(&other.draft) || self.draft == other.draft)
    }
    pub fn is_current(&self, writer: Writer) -> bool {
        Self::capture(writer).is_some_and(|current| self.matches(&current))
    }
}
pub(super) struct Completed {
    pub request: Arc<Request>,
    pub items: SourceCompletions,
}
impl PartialEq for Completed {
    fn eq(&self, other: &Self) -> bool {
        // Results are immutable; memo refreshes follow publication identity.
        std::ptr::eq(self, other)
    }
}
