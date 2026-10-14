//! One completion result belongs to one accepted document, draft and cursor.
use crate::editing::Writer;
use freya::{prelude::*, text_edit::TextEditor};
use recite_core::SourcePosition;
use recite_writer_model::SourceCompletions;
use std::sync::Arc;

pub(crate) struct SourceCompletionCache {
    document: Arc<str>,
    revision: i64,
    position: SourcePosition,
    items: Arc<SourceCompletions>,
}
impl SourceCompletionCache {
    fn matches(
        &self,
        document: &Arc<str>,
        revision: i64,
        position: SourcePosition,
        draft: &str,
    ) -> bool {
        Arc::ptr_eq(&self.document, document)
            && self.revision == revision
            && self.position == position
            && self.items.source() == draft
    }
}

pub(super) fn candidates(writer: Writer) -> Option<Arc<SourceCompletions>> {
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
    let draft = editor.rope.to_string();
    let model = writer.buffers.model.read();
    let document = model.as_ref().ok()?.document();
    let source = document.source_snapshot();
    let revision = document.revision();
    let mut cache = writer.source_viewport.cache;
    if let Some(previous) = cache.peek().as_ref()
        && previous.matches(&source, revision, position, &draft)
    {
        return Some(previous.items.clone());
    }
    let items = Arc::new(document.complete_draft(&draft, position).ok()?);
    cache.set(Some(SourceCompletionCache {
        document: source,
        revision,
        position,
        items: items.clone(),
    }));
    Some(items)
}

#[cfg(test)]
mod tests;
