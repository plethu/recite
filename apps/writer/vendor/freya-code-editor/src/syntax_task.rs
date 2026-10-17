//! Component-scoped syntax publication; workers wake only when output is ready.
use crate::editor_data::CodeEditorData;
use freya_components::scrollviews::ScrollController;
use freya_core::prelude::*;

pub(crate) fn use_syntax_work(
    editor: Writable<CodeEditorData>,
    scroll: ScrollController,
    line_height: f32,
) {
    let current_editor = use_reactive(&editor);
    let viewport = use_reactive(&(scroll, line_height));
    use_future(move || {
        let mut editor = current_editor.read().clone();
        let receiver = editor.read().syntax_notifications();
        async move {
            let Some(receiver) = receiver else {
                return;
            };
            loop {
                let closed = receiver.recv().await.is_err();
                editor.write_if(|mut data| data.receive_syntax(&receiver, closed));
                if closed {
                    break;
                }
            }
        }
    });
    use_after_side_effect(move || {
        let mut editor = current_editor.read().clone();
        let (scroll, line_height) = *viewport.read();
        let rows = {
            let data = editor.read();
            let (_, y): (i32, i32) = scroll.into();
            let first = (-(y as f32) / line_height).floor().max(0.) as usize;
            let count = (data.viewport.height / line_height).ceil().max(0.) as usize;
            first.saturating_sub(4).min(data.rope.len_lines())
                ..first
                    .saturating_add(count)
                    .saturating_add(5)
                    .min(data.rope.len_lines())
        };
        editor.write_if(|mut data| data.visible_rows(rows));
    });
}
