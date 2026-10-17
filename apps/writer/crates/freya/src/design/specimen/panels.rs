//! Print specimen sections built from production surfaces.
use super::super::tokens::PrintTypography;
use crate::design::{BeatCard, Button, palette, tokens as t};
use freya::prelude::*;

fn slug() -> Rect {
    use t::Slug::{Flag, Meta};
    t::slug([
        Meta("relay_hub.recite".into()),
        Meta(":: relay_desk".into()),
        Flag("default".into()),
        Meta("4 replies".into()),
    ])
}

pub(super) fn elevation(p: &palette::Palette) -> Rect {
    // Elevation is a hard offset plate: teal for menus and cards, plum for the selected card
    // and dialogs. The empty state and loading mark share the print vocabulary.
    let menu = rect()
        .width(Size::px(220.))
        .padding(t::SPACE_XS)
        .corner_radius(t::DIALOG_RADIUS)
        .background(p.floating)
        .border(Border::new().width(1.).fill(p.rule))
        .shadow(super::super::material::offset(p.teal, t::SHADOW_OFFSET))
        .child(
            Button::new()
                .menu_item()
                .width(Size::fill())
                .child("Line details"),
        )
        .child(
            Button::new()
                .menu_item()
                .width(Size::fill())
                .child("Add reply"),
        );
    let dialog = rect()
        .width(Size::px(320.))
        .padding(t::SPACE_LG)
        .spacing(t::SPACE_MD)
        .corner_radius(t::DIALOG_RADIUS)
        .background(p.floating)
        .border(Border::new().width(t::STROKE_WIDTH).fill(p.stamp))
        .shadow(super::super::material::offset(
            p.accent,
            t::DIALOG_SHADOW_OFFSET,
        ))
        .child(
            label()
                .text("Unsaved changes")
                .display_font()
                .font_size(t::title()),
        )
        .child(
            rect()
                .horizontal()
                .spacing(t::SPACE_SM)
                .child(Button::new().stamp().child("Keep editing"))
                .child(Button::new().filled().child("Save and close")),
        );
    rect()
        .width(Size::fill())
        .spacing(t::SPACE_LG)
        .child(t::caps("Elevation and states"))
        .child(
            rect()
                .horizontal()
                .spacing(t::SPACE_XL)
                .cross_align(Alignment::Start)
                .child(menu)
                .child(dialog)
                .child(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .spacing(t::SPACE_SM)
                        .child(super::super::material::loading_mark(false))
                        .child(t::meta("opening project…")),
                ),
        )
        .child(super::super::EmptyState {
            title: "No beats yet".into(),
            instruction: "Add a beat, or open the Source view and type one.".into(),
            actions: Vec::new(),
            min_height: 200.,
        })
}

pub(super) fn manuscript(mut picker_open: State<bool>) -> Rect {
    rect().width(Size::fill()).horizontal().content(Content::Flex).spacing(t::SPACE_XL)
        .child(rect().width(Size::flex(1.)).spacing(t::SPACE_MD)
            .child(rect().spacing(t::SPACE_XS)
                .child(label().text("Relay Desk").display_font().font_size(t::title() * 1.5))
                .child(slug()))
            .child(rect().width(Size::fill()).spacing(t::SPACE_XS)
                .child(t::caps("Mara"))
            .child(t::prose().width(Size::fill()).child(label()
                .text("If you're here about the transmitter, take a number. If you're here about the smoke, take a bucket.")
                .line_height(1.5))))
            .child(rect().width(Size::fill()).spacing(t::SPACE_XS)
                .child(t::caps("Reply 1"))
            .child(t::prose().child(label().text("Anything I can do to help?").line_height(1.5))))
            .child(Button::new().flat().on_press(move |_| picker_open.set(true)).child("Missing Courier →")))
        .child(rect().width(Size::px(320.)).spacing(t::SPACE_MD)
            .child(t::caps("Graph cards"))
            .child(rect().width(Size::fill()).height(Size::px(t::CARD_HEIGHT)).child(BeatCard {
                title: "Relay Desk".into(), speaker: "Mara".into(),
                excerpt: "If you're here about the transmitter, take a number. If you're here about the smoke, take a bucket.".into(),
                annotations: String::new(), selected: true, active: false, zoom: 1.,
            }))
            .child(rect().width(Size::fill()).height(Size::px(t::CARD_HEIGHT)).child(BeatCard {
                title: "Missing Courier".into(), speaker: "Mara".into(),
                excerpt: "Our courier is two days late. Everyone keeps asking about the battery she was carrying.".into(),
                annotations: String::new(), selected: false, active: false, zoom: 1.,
            })))
}
