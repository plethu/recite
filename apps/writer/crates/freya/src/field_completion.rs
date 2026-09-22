//! Completion retains Source input ownership and compiler-provided replacement spans.
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    messages::{MsgId, text},
};
use freya::{prelude::*, text_edit::TextEditor};
use recite_writer_model::SourceCompletions;

#[derive(Clone)]
pub(super) struct Completion {
    pub writer: Writer,
    pub editor_id: AccessibilityId,
    pub viewport: State<Option<Area>>,
}
impl PartialEq for Completion {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
    }
}
fn candidates(writer: Writer) -> Option<SourceCompletions> {
    let editor = writer.buffers.editor.read();
    let offset = editor
        .rope
        .utf16_cu_to_char(editor.cursor_pos().min(editor.rope.len_utf16_cu()));
    let line = editor.rope.char_to_line(offset);
    let column = offset - editor.rope.line_to_char(line);
    let position = recite_core::SourcePosition::new(
        u32::try_from(line + 1).ok()?,
        u32::try_from(column + 1).ok()?,
    )
    .ok()?;
    writer
        .buffers
        .model
        .read()
        .as_ref()
        .ok()?
        .document()
        .complete_draft(&editor.rope.to_string(), position)
        .ok()
}
pub(crate) fn keyboard(mut writer: Writer, event: &Event<KeyboardEventData>) -> bool {
    let mut state = writer.source_viewport;
    if event.code == Code::Space && event.modifiers == Modifiers::CONTROL {
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
                let count = candidates(writer).map_or(0, |items| items.candidates().len());
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
impl Component for Completion {
    fn render(&self) -> impl IntoElement {
        let colors = t::colors();
        let scroll = use_scroll_controller(ScrollConfig::default);
        let writer = self.writer;
        let editor_id = self.editor_id;
        let mut state = writer.source_viewport;
        use_after_side_effect(move || {
            let opened = *state.completion.read();
            if opened && *Platform::get().focused_accessibility_id.read() != editor_id {
                editor_id.request_focus();
            }
        });
        let items = use_memo(move || {
            if *state.completion.read() {
                candidates(writer)
            } else {
                None
            }
        });
        crate::design::use_list_reveal(None, state.candidate, scroll, t::picker_row_height(), 4);
        let mut root = rect().child(
            Button::new()
                .flat()
                .named(text(MsgId::WriterCompleteSource))
                .on_press(move |_| {
                    state.completion.set(true);
                    state.candidate.set(None);
                    editor_id.request_focus();
                })
                .child(text(MsgId::WriterGuiCompleteCtrlSpace)),
        );
        if !*state.completion.read() {
            return root.into_element();
        }
        let Some(area) = *self.viewport.read() else {
            return root.into_element();
        };
        let editor = writer.buffers.editor.read();
        let size = t::code_size();
        let caret = crate::source_editor::caret_rect(&editor, size);
        let (x, y): (i32, i32) = state.scroll.into();
        let width = (t::PICKER_MAX_WIDTH * t::ui_scale()).min(area.width());
        let line = (size * t::CODE_LINE_HEIGHT).floor();
        let left = (area.min_x() + size * 5. + caret.left + x as f32)
            .clamp(area.min_x(), (area.max_x() - width).max(area.min_x()));
        let bottom = area.min_y() + (editor.cursor_row() + 1) as f32 * line + y as f32;
        let count = items.read().as_ref().map_or(0, |i| i.candidates().len());
        let height = (count.clamp(1, 6) as f32 * t::picker_row_height()).min(area.height());
        let top = if bottom + height <= area.max_y() {
            bottom
        } else {
            (bottom - line - height).max(area.min_y())
        };
        drop(editor);
        let mut popup = rect()
            .position(Position::new_global().left(left).top(top))
            .width(Size::px(width))
            .height(Size::px(height))
            .layer(Layer::Overlay)
            .a11y_role(AccessibilityRole::ListBox)
            .a11y_alt(text(MsgId::WriterCompleteSource))
            .background(colors.inset)
            .corner_radius(t::RADIUS)
            .border(Border::new().width(1.).fill(colors.boundary));
        if let Some(items) = items.read().as_ref() {
            let entries = items.clone();
            let active = *state.candidate.read();
            popup = popup.child(
                VirtualScrollView::new_with_data(entries, move |row, entries| {
                    let entries = entries.clone();
                    let name = entries.candidates()[row.index].name().to_owned();
                    Button::new()
                        .flat()
                        .width(Size::fill())
                        .named(name.clone())
                        .selected(active == Some(row.index))
                        .on_press(move |_| {
                            let mut writer = writer;
                            if let Err(error) = apply(writer, &entries, row.index) {
                                writer.message.error(error);
                            }
                            state.completion.set(false);
                            editor_id.request_focus();
                        })
                        .child(label().width(Size::fill()).text(name))
                        .into_element()
                })
                .length(count)
                .item_size(t::picker_row_height())
                .height(Size::fill())
                .scroll_controller(scroll),
            );
        }
        if count == 0 {
            popup = popup.child(label().text(text(MsgId::WriterNoCompletions)));
        }
        root = root.child(popup);
        root.into_element()
    }
}
fn apply(writer: Writer, items: &SourceCompletions, index: usize) -> Result<(), String> {
    let mut state = writer.buffers.editor;
    let mut editor = state.write();
    let source = editor.rope.to_string();
    let (range, replacement) = items
        .replacement(&source, index)
        .map_err(|e| e.to_string())?;
    let start = source[..range.start].encode_utf16().count();
    let end = source[..range.end].encode_utf16().count();
    editor.remove(start..end);
    // Freya holds one pending syntax edit; consume it before the insertion.
    editor.parse();
    editor.insert(replacement, start);
    editor.clear_selection();
    editor.move_cursor_to(start + replacement.encode_utf16().count());
    writer.source_viewport.request();
    editor.parse();
    editor.measure(t::code_size(), "monospace");
    Ok(())
}
