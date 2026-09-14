//! Preview is opened deliberately and never duplicates the writing surface by default.
use crate::{editing::Writer, palette};
use freya::prelude::*;

pub(super) fn render(writer: Writer, mut visible: State<bool>) -> Element {
    let state = writer.buffers.model.peek();
    let Some(page) = state
        .as_ref()
        .ok()
        .and_then(|session| session.preview_page())
    else {
        return rect().into_element();
    };
    let mut panel = rect()
        .width(Size::px(280.))
        .padding(20.)
        .spacing(16.)
        .border(
            Border::new()
                .width(BorderWidth {
                    left: 1.,
                    ..Default::default()
                })
                .fill(palette::rule(writer.dark)),
        )
        .child(label().text("Try this scene").font_size(18.))
        .child(
            Button::new()
                .cursor_icon(CursorIcon::Pointer)
                .flat()
                .on_press(move |_| visible.set(false))
                .child("Close preview"),
        )
        .child(
            label()
                .text(page.text.clone())
                .font_family("serif")
                .font_size(20.),
        );
    for (index, choice) in page.choices.iter().enumerate() {
        panel = panel.child(
            Button::new()
                .cursor_icon(CursorIcon::Pointer)
                .on_press(move |_| writer.perform(|m| m.advance_preview(Some(index))))
                .child(label().text(choice.text.clone())),
        );
    }
    if page.choices.is_empty() && !page.ended {
        panel = panel.child(
            Button::new()
                .cursor_icon(CursorIcon::Pointer)
                .filled()
                .on_press(move |_| writer.perform(|m| m.advance_preview(None)))
                .child("Continue"),
        );
    }
    panel.into_element()
}
