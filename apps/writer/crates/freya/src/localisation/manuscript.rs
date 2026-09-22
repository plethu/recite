//! Read-only source and editable translation share the same column geometry.
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    palette,
};
use freya::prelude::*;
use recite_writer_model::{Passage, PassageKind};
use t::ProseTypography;

pub(crate) fn columns(dark: bool, source: Element, translation: Element) -> Element {
    rect()
        .horizontal()
        .content(Content::Flex)
        .width(Size::fill())
        .child(
            rect()
                .width(Size::flex(1.))
                .padding((0., t::SPACE_LG, 0., 0.))
                .child(source),
        )
        .child(
            rect()
                .width(Size::flex(1.))
                .padding((0., 0., 0., t::SPACE_LG))
                .border(
                    Border::new()
                        .width(BorderWidth {
                            left: 1.,
                            ..Default::default()
                        })
                        .fill(palette::rule(dark)),
                )
                .child(translation),
        )
        .into_element()
}

pub(crate) fn source_passage(writer: Writer, passage: &Passage, reply: Option<usize>) -> Element {
    let caption = match &passage.kind {
        PassageKind::Dialogue { speaker } => {
            palette::display_name(speaker.as_deref().unwrap_or("Narration"))
        }
        PassageKind::Choice { .. } => {
            reply.map_or_else(|| "Reply".into(), |n| format!("Reply {n}"))
        }
    };
    rect()
        .width(Size::fill())
        .spacing(t::SPACE_XS)
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt(format!("Source · {caption} · read only"))
        .child(
            rect()
                .height(Size::px(t::prose_meta_height()))
                .main_align(Alignment::Center)
                .child(
                    label()
                        .text(caption)
                        .font_size(t::small())
                        .color(palette::muted(writer.dark)),
                ),
        )
        .child(
            label()
                .width(Size::fill())
                .text(passage.text.clone())
                .prose_font()
                .font_size(t::heading()),
        )
        .into_element()
}

pub(super) fn header(mut writer: Writer) -> Element {
    let beat = writer
        .buffers
        .model
        .read()
        .as_ref()
        .ok()
        .and_then(|m| m.selected_block().ok().flatten());
    let Some(beat) = beat else {
        return rect().into_element();
    };
    let language = writer.localisation.read().catalogue.as_ref().map(|c| {
        c.document
            .headers()
            .iter()
            .find(|h| h.key() == "Language")
            .map_or_else(
                || super::messages::text(super::messages::MsgId::WriterTranslation),
                |h| format!("Translation · {}", h.value()),
            )
    });
    let source = rect()
        .horizontal()
        .width(Size::fill())
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .child(
            label()
                .width(Size::flex(1.))
                .text(crate::messages::text(
                    crate::messages::MsgId::WriterGuiSourceReadOnly,
                ))
                .font_size(t::small())
                .color(palette::muted(writer.dark)),
        )
        .child(
            Button::new()
                .flat()
                .named(crate::messages::text(
                    crate::messages::MsgId::WriterGuiEditSource,
                ))
                .on_press(move |_| {
                    if writer.try_navigate(|_| Ok(())).is_ok() {
                        writer.localisation.write().active = false;
                        writer.pane.set(crate::editing::Pane::Script);
                        writer.inspector_focus.request_focus();
                    }
                })
                .child(crate::messages::text(
                    crate::messages::MsgId::WriterGuiEditSource,
                )),
        );
    rect()
        .width(Size::fill())
        .padding((t::SPACE_XS, t::SPACE_SM))
        .child(
            rect()
                .width(Size::fill())
                .max_width(Size::px(1320.))
                .spacing(t::SPACE_XS)
                .child(
                    label()
                        .text(palette::display_name(&beat))
                        .prose_font()
                        .font_size(t::title()),
                )
                .child(super::context::Context { writer, beat })
                .child(if let Some(language) = language {
                    columns(
                        writer.dark,
                        source.into_element(),
                        rect()
                            .height(Size::px(t::control_height()))
                            .main_align(Alignment::Center)
                            .child(label().text(language).font_size(t::small()))
                            .into_element(),
                    )
                } else {
                    source.into_element()
                }),
        )
        .border(
            Border::new()
                .width(BorderWidth {
                    bottom: 1.,
                    ..Default::default()
                })
                .fill(palette::rule(writer.dark)),
        )
        .into_element()
}
