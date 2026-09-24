//! Explicit, immediately applied reading and interface sizes.
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    messages::{MsgId, text},
};
use freya::prelude::*;
use recite_config::WriterPresentationField as Field;

pub(super) fn controls(writer: Writer, ids: [AccessibilityId; 6]) -> Element {
    let current = writer.preferences.read().config.writer.presentation;
    let mut content = rect().width(Size::fill()).spacing(t::SPACE_SM).child(
        label()
            .text(text(MsgId::WriterWorkspacePresentation))
            .font_size(t::small())
            .color(crate::design::palette::current().muted),
    );
    for (index, field, caption, step) in [
        (0, Field::ReadingSize, MsgId::WriterWorkspaceReadingSize, 1),
        (1, Field::SourceSize, MsgId::WriterWorkspaceSourceSize, 1),
        (2, Field::UiScale, MsgId::WriterWorkspaceUiScale, 25),
    ] {
        let value = current.value(field);
        let (min, max) = field.bounds();
        let name = text(caption);
        let mut row = rect()
            .horizontal()
            .width(Size::fill())
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(t::SPACE_SM)
            .child(label().text(name.clone()).width(Size::flex(1.)));
        let mut stepper = rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(t::SPACE_XS);
        for (side, delta, caption) in [
            (0, -step, MsgId::WriterWorkspaceDecrease),
            (1, step, MsgId::WriterWorkspaceIncrease),
        ] {
            let next = (i32::from(value) + delta).clamp(i32::from(min), i32::from(max)) as u16;
            if side == 1 {
                stepper = stepper.child(
                    label()
                        .text(format!(
                            "{value}{}",
                            if field == Field::UiScale { "%" } else { " px" }
                        ))
                        .width(Size::px(64. * t::ui_scale()))
                        .text_align(TextAlign::Center),
                );
            }
            stepper = stepper.child(
                Button::new()
                    .width(Size::px(t::control_height()))
                    .a11y_id(ids[index * 2 + side])
                    .named(format!("{} {name}", text(caption)))
                    .enabled(next != value)
                    .child(if side == 0 { "−" } else { "+" })
                    .on_press(move |_| {
                        let current = writer.preferences.peek().config.writer.presentation;
                        if let Ok(next) = current.with_value(field, next) {
                            crate::presentation::persist(writer, next);
                        }
                    }),
            );
        }
        row = row.child(stepper);
        content = content.child(row);
    }
    content.into_element()
}
