//! Preview is opened deliberately and never duplicates the writing surface by default.
use crate::design::Button;
use crate::design::tokens as t;
use crate::{editing::Writer, palette};
use freya::prelude::*;

pub(super) fn render(writer: Writer) -> Element {
    let mut pane = writer.pane;
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
        .spacing(t::SPACE_LG)
        .border(
            Border::new()
                .width(BorderWidth {
                    left: 1.,
                    ..Default::default()
                })
                .fill(palette::rule(writer.dark)),
        )
        .child(label().text("Try this scene").font_size(t::TEXT_HEADING))
        .child(
            Button::new()
                .flat()
                .on_press(move |_| pane.set(crate::editing::Pane::Map))
                .child("Close preview"),
        )
        .child(
            label()
                .text(page.text.clone())
                .font_family("serif")
                .font_size(t::TEXT_PROSE),
        );
    for effect in &page.effects {
        panel = panel.child(
            label()
                .text(format!(
                    "{}({:?}) · request only",
                    effect.function, effect.args
                ))
                .font_size(t::TEXT_SMALL),
        );
    }
    for (index, choice) in page.choices.iter().enumerate() {
        panel = panel.child(
            Button::new()
                .on_press(move |_| writer.perform(|m| m.advance_preview(Some(index))))
                .child(label().text(choice.text.clone())),
        );
    }
    if page.choices.is_empty() && !page.ended {
        panel = panel.child(
            Button::new()
                .filled()
                .on_press(move |_| writer.perform(|m| m.advance_preview(None)))
                .child("Continue"),
        );
    }
    panel.into_element()
}
