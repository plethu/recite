//! Diagnostics keep their source coordinates and navigate through the draft guard.
mod panel;
pub(crate) use panel::Panel;

use crate::{
    design::{Button, tokens as t},
    editing::Writer,
};
use freya::{code_editor::*, prelude::*, text_edit::TextEditor};
use recite_core::{Diagnostic, SourcePosition};

fn row(
    writer: Writer,
    editor_id: AccessibilityId,
    id: AccessibilityId,
    diagnostic: &Diagnostic,
) -> Element {
    let span = diagnostic.span.clone();
    let available = writer
        .buffers
        .model
        .peek()
        .as_ref()
        .is_ok_and(|model| model.document().key().as_str() == span.file)
        || writer
            .files
            .peek()
            .as_ref()
            .is_some_and(|files| files.path_for_document(&span.file).is_some());
    let caption_file = span.file.clone();
    Button::new()
        .flat()
        .width(Size::fill())
        .enabled(available)
        .named(format!(
            "Open diagnostic {} at {}:{}:{}",
            diagnostic.code,
            span.file,
            span.start.line(),
            span.start.column()
        ))
        .a11y_id(id)
        .described(diagnostic.message.clone())
        .on_press(move |_| open(writer, editor_id, &span))
        .child(
            rect()
                .width(Size::fill())
                .child(
                    label()
                        .text(diagnostic.message.clone())
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis),
                )
                .child(
                    label()
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis)
                        .font_size(t::small())
                        .text(format!(
                            "{} · {}:{}:{}",
                            diagnostic.code,
                            caption_file,
                            diagnostic.span.start.line(),
                            diagnostic.span.start.column()
                        )),
                ),
        )
        .into_element()
}

fn open(writer: Writer, editor_id: AccessibilityId, span: &recite_core::SourceSpan) {
    let position = span.start;
    let location = crate::navigation::location::Location {
        screen: crate::navigation::location::Screen::Write,
        beat: None,
        passage: None,
        document: span.file.clone(),
        source: true,
        ..crate::navigation::snapshot(writer)
    };
    let mut message = writer.message;
    let result = crate::navigation::route(
        writer,
        location,
        None,
        EventHandler::new(move |result: Result<(), String>| {
            if let Err(error) = result {
                message.error(error);
                return;
            }
            let mut editor_state = writer.buffers.editor;
            let mut editor = editor_state.write();
            let offset = position_offset(&editor.rope, position);
            editor.clear_selection();
            editor.move_cursor_to(offset);
            writer.source_viewport.request();
            drop(editor);
            editor_id.request_focus();
        }),
    );
    if let Err(error) = result {
        message.error(error);
    }
}

fn position_offset(rope: &Rope, position: SourcePosition) -> usize {
    let line = (position.line() as usize - 1).min(rope.len_lines().saturating_sub(1));
    let column = (position.column() as usize - 1).min(rope.line(line).len_chars());
    rope.char_to_utf16_cu((rope.line_to_char(line) + column).min(rope.len_chars()))
}

#[cfg(test)]
mod tests;
