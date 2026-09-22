//! Diagnostics keep their source coordinates and navigate through the draft guard.
use crate::{
    design::{Button, tokens as t},
    editing::{Pane, Writer},
};
use freya::{code_editor::*, prelude::*, text_edit::TextEditor};
use recite_core::{Diagnostic, SourcePosition};
use recite_writer_model::View;

pub(crate) fn row(writer: Writer, editor_id: AccessibilityId, diagnostic: &Diagnostic) -> Element {
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
        .on_press(move |_| {
            let mut writer = writer;
            let current = writer
                .buffers
                .model
                .peek()
                .as_ref()
                .ok()
                .map(|m| m.document().key().to_string());
            let select = |model: &mut recite_writer_model::Workbench| {
                if model.view() == &View::Source {
                    Ok(())
                } else {
                    model.select(View::Source)
                }
            };
            let result = if current.as_deref() == Some(span.file.as_str()) {
                writer.try_navigate(select)
            } else {
                let path = writer
                    .files
                    .peek()
                    .as_ref()
                    .and_then(|files| files.path_for_document(&span.file));
                match path {
                    Some(path) => writer
                        .buffers
                        .switch(writer.files, &path, writer.dark, select),
                    None => Err("The diagnostic document is not in this project.".into()),
                }
            };
            if let Err(error) = result {
                writer.message.error(error);
                return;
            }
            writer.pane.set(Pane::Map);
            writer.layout.view.set(recite_config::WriterView::Source);
            let mut editor = writer.buffers.editor.write();
            let offset = position_offset(&editor.rope, span.start);
            editor.clear_selection();
            editor.move_cursor_to(offset);
            writer.source_viewport.request();
            drop(editor);
            editor_id.request_focus();
        })
        .child(
            rect()
                .width(Size::fill())
                .child(label().text(diagnostic.message.clone()))
                .child(label().font_size(t::small()).text(format!(
                    "{} · {}:{}:{}",
                    diagnostic.code,
                    caption_file,
                    span.start.line(),
                    span.start.column()
                ))),
        )
        .into_element()
}

fn position_offset(rope: &Rope, position: SourcePosition) -> usize {
    let line = (position.line() as usize - 1).min(rope.len_lines().saturating_sub(1));
    let column = (position.column() as usize - 1).min(rope.line(line).len_chars());
    rope.char_to_utf16_cu((rope.line_to_char(line) + column).min(rope.len_chars()))
}

#[cfg(test)]
mod tests;
