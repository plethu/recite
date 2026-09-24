//! Host-owned caret reveal and geometry around the released Freya editor.
mod surface;
use freya::{
    code_editor::CodeEditorData, elements::paragraph::ParagraphCursorExt, prelude::*,
    text_edit::TextEditor,
};
use skia_safe::textlayout::{FontCollection, ParagraphBuilder, ParagraphStyle, TextStyle};
pub(crate) use surface::EditorSurface;

#[derive(Clone, Copy)]
pub(crate) struct EditorViewport {
    pub completion: State<bool>,
    pub candidate: State<Option<usize>>,
    pub scroll: ScrollController,
    pub reveal: State<bool>,
}
impl EditorViewport {
    pub fn new() -> Self {
        Self {
            completion: use_state(|| false),
            candidate: use_state(|| None),
            scroll: use_scroll_controller(ScrollConfig::default),
            reveal: use_state(|| false),
        }
    }
    pub fn request(mut self) {
        self.reveal.set(true);
    }
    pub fn observe(self, editor: State<CodeEditorData>, size: f32) {
        let size = use_reactive(&size);
        let mut previous = use_state(|| None);
        let mut pending = self.reveal;
        let mut scroll = self.scroll;
        use_after_side_effect(move || {
            let size = *size.read();
            let data = editor.read();
            let cursor = data.cursor_pos();
            let changed = previous.peek().is_some_and(|old| old != (cursor, size));
            if changed {
                pending.set_if_modified(true);
            }
            previous.set_if_modified(Some((cursor, size)));
            if !*pending.read() || data.viewport.width <= 0. || data.viewport.height <= 0. {
                return;
            }
            data.scroll_to_cursor(
                scroll,
                (size * super::design::tokens::CODE_LINE_HEIGHT).floor(),
            );
            let caret = caret_rect(&data, size);
            let (x, y) = scroll.into();
            let left = caret.left;
            let right = caret.right.max(caret.left + 6.);
            let line = (size * super::design::tokens::CODE_LINE_HEIGHT).floor();
            scroll.scroll_to_x(reveal_axis(x, left, right, data.viewport.width - 16.));
            scroll.scroll_to_y(reveal_axis(
                y,
                data.cursor_row() as f32 * line,
                (data.cursor_row() + 1) as f32 * line,
                data.viewport.height - 16.,
            ));
            pending.set(false);
        });
    }
}

pub(crate) fn caret_rect(editor: &CodeEditorData, size: f32) -> skia_safe::Rect {
    let text = editor.rope.line(editor.cursor_row()).to_string();
    let mut font = TextStyle::default();
    font.set_font_size(size);
    font.set_font_families(&["monospace"]);
    let mut style = ParagraphStyle::default();
    style.set_text_style(&font);
    let mut builder = ParagraphBuilder::new(&style, consume_root_context::<FontCollection>());
    builder.add_text(&text);
    let mut paragraph = builder.build();
    paragraph.layout(f32::MAX);
    paragraph.cursor_rect(&text, editor.cursor_col(), TextAlign::Left)
}
fn reveal_axis(scroll: i32, start: f32, end: f32, extent: f32) -> i32 {
    if start < -(scroll as f32) {
        -start.floor().max(0.) as i32
    } else if end > -(scroll as f32) + extent {
        -(end - extent).ceil().max(0.) as i32
    } else {
        scroll
    }
}

#[cfg(test)]
mod tests;
