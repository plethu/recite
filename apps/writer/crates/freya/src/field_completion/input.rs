//! Keyboard and pointer activation recheck request ownership before changing text.
use super::{cache, cache::candidates, request::Completed};
use crate::{design::tokens as t, editing::Writer};
use freya::{prelude::*, text_edit::TextEditor};
use recite_writer_model::SourceCompletions;

pub(crate) fn keyboard(mut writer: Writer, event: &Event<KeyboardEventData>) -> bool {
    let mut state = writer.source_viewport;
    if event.code == Code::Space && event.modifiers == Modifiers::CONTROL {
        cache::clear(writer);
        state.completion.set(true);
        state.candidate.set(None);
    } else if *state.completion.peek() {
        if matches!(event.key, Key::Named(NamedKey::Dead | NamedKey::Process)) {
            state.completion.set(false);
            return false;
        }
        match event.key {
            Key::Named(NamedKey::Escape) => state.completion.set(false),
            Key::Named(NamedKey::ArrowDown | NamedKey::ArrowUp) if event.modifiers.is_empty() => {
                let count = candidates(writer).map_or(0, |items| items.items.candidates().len());
                if count > 0 {
                    let current = *state.candidate.peek();
                    state.candidate.set(Some(match (current, &event.key) {
                        (Some(i), Key::Named(NamedKey::ArrowUp)) => (i + count - 1) % count,
                        (Some(i), _) => (i + 1) % count,
                        (None, _) => 0,
                    }));
                }
            }
            Key::Named(NamedKey::Enter)
                if event.modifiers.is_empty() && state.candidate.peek().is_some() =>
            {
                if let Some(items) = candidates(writer)
                    && let Some(index) = *state.candidate.peek()
                    && let Err(error) = apply(writer, &items, index)
                {
                    writer.message.error(error);
                }
                state.completion.set(false);
            }
            Key::Named(
                NamedKey::Tab
                | NamedKey::ArrowLeft
                | NamedKey::ArrowRight
                | NamedKey::Home
                | NamedKey::End,
            ) => {
                state.completion.set(false);
                return false;
            }
            _ => {
                state.candidate.set(None);
                return false;
            }
        }
    } else {
        return false;
    }
    event.prevent_default();
    event.stop_propagation();
    true
}
pub(super) fn apply(writer: Writer, completed: &Completed, index: usize) -> Result<(), String> {
    if !*writer.source_viewport.completion.peek()
        || !completed.request.is_current(writer)
        || !writer
            .source_viewport
            .cache
            .peek()
            .as_ref()
            .is_some_and(|cache| cache.owns(completed))
    {
        return Err(recite_writer_model::EditError::Stale.to_string());
    }
    let items = &completed.items;
    let mut state = writer.buffers.editor;
    let mut editor = state.write();
    let (range, replacement) =
        utf16_replacement(&editor.rope, items, index).map_err(|e| e.to_string())?;
    let start = range.start;
    editor.remove(range);
    editor.insert(replacement, start);
    editor.clear_selection();
    editor.move_cursor_to(start + replacement.encode_utf16().count());
    writer.source_viewport.request();
    editor.parse();
    editor.measure(t::code_size(), "monospace");
    Ok(())
}

fn utf16_replacement<'a>(
    rope: &freya::code_editor::Rope,
    items: &'a SourceCompletions,
    index: usize,
) -> Result<(std::ops::Range<usize>, &'a str), recite_writer_model::EditError> {
    if rope.slice(..) != items.source() {
        return Err(recite_writer_model::EditError::Stale);
    }
    let (range, replacement) = items.replacement(items.source(), index)?;
    let start = rope.char_to_utf16_cu(rope.byte_to_char(range.start));
    let end = rope.char_to_utf16_cu(rope.byte_to_char(range.end));
    Ok((start..end, replacement))
}

#[cfg(test)]
mod tests;
