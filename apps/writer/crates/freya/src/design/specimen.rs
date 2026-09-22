//! Runnable native specimen of production controls. No project or preferences are written.
use super::{
    BeatCard, Button, PickerOption, ReducedMotion, SearchField, SearchPicker, Segments, palette,
    tokens as t,
};
use freya::prelude::*;
use std::sync::Arc;

pub(crate) fn app() -> Element {
    let mut dark = use_state(|| true);
    let mut theme = use_init_theme(|| palette::theme(true));
    let mut reduced = use_state(|| false);
    use_provide_context(|| ReducedMotion(reduced));
    let query = use_state(String::new);
    let active = use_state(|| None);
    let picker_query = use_state(String::new);
    let mut picker_open = use_state(|| false);
    let mut destination = use_state(|| "Missing Courier".to_owned());
    let mut view = use_state(|| 0);
    let mut message = use_state(|| "Try the controls, then compare both appearances.".to_owned());
    let options = use_memo(move || {
        let needle = picker_query.read().to_lowercase();
        Arc::new(
            ["Missing Courier", "Station History", "Goodbye"]
                .into_iter()
                .filter(|title| title.to_lowercase().contains(&needle))
                .map(|title| PickerOption {
                    annotation: String::new(),
                    value: title.into(),
                    title: title.into(),
                    detail: String::new(),
                })
                .collect::<Vec<_>>(),
        )
    });
    let p = t::colors();
    let header = rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .spacing(t::SPACE_SM)
        .child(
            label()
                .text("recite.")
                .font_size(26.)
                .font_weight(FontWeight::MEDIUM),
        )
        .child(label().text("Native component specimen").color(p.muted))
        .child(rect().width(Size::flex(1.)))
        .child(
            Button::new()
                .flat()
                .checkable(*reduced.read())
                .selected(*reduced.read())
                .on_press(move |_| {
                    let next = !*reduced.peek();
                    reduced.set(next);
                })
                .child("Reduced motion"),
        )
        .child(
            Button::new()
                .on_press(move |_| {
                    let next = !*dark.peek();
                    dark.set(next);
                    theme.set(palette::theme(next));
                })
                .child(if *dark.read() {
                    "Light appearance"
                } else {
                    "Dark appearance"
                }),
        );
    let controls = rect()
        .width(Size::fill())
        .spacing(t::SPACE_MD)
        .child(
            label()
                .text("Actions and navigation")
                .font_weight(FontWeight::MEDIUM),
        )
        .child(
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(t::SPACE_SM)
                .child(
                    Button::new()
                        .filled()
                        .on_press(move |_| message.set("Primary action activated.".into()))
                        .child("Try scene"),
                )
                .child(
                    Button::new()
                        .on_press(move |_| message.set("Secondary action activated.".into()))
                        .child("Add beat"),
                )
                .child(
                    Button::new()
                        .flat()
                        .on_press(move |_| message.set("Quiet action activated.".into()))
                        .child("Arrange automatically"),
                )
                .child(Button::new().enabled(false).child("Save changes"))
                .child(Segments {
                    name: "Writing view".into(),
                    labels: ["Map".into(), "Source".into()],
                    ids: [use_a11y(), use_a11y()],
                    selected: *view.read(),
                    vim: false,
                    width: Size::px(180.),
                    change: EventHandler::new(move |index| view.set(index)),
                }),
        )
        .child(
            rect()
                .width(Size::fill())
                .horizontal()
                .content(Content::Flex)
                .spacing(t::SPACE_XL)
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .spacing(t::SPACE_SM)
                        .child(
                            label()
                                .text("Find a beat")
                                .font_size(t::small())
                                .color(p.muted),
                        )
                        .child(SearchField {
                            query,
                            id: use_a11y(),
                            placeholder: "Search scene names…".into(),
                            active,
                            count: 0,
                            vim: false,
                            activate: EventHandler::new(|_| {}),
                            changed: EventHandler::new(|()| {}),
                        }),
                )
                .child(
                    rect()
                        .width(Size::flex(1.))
                        .spacing(t::SPACE_SM)
                        .child(
                            label()
                                .text("Destination")
                                .font_size(t::small())
                                .color(p.muted),
                        )
                        .child(SearchPicker {
                            id: use_a11y(),
                            input_id: use_a11y(),
                            name: "Change destination".into(),
                            placeholder: "Find a destination…".into(),
                            empty_hint: "Choose a beat".into(),
                            no_matches: "No matching destinations".into(),
                            selected: destination.read().clone(),
                            query: picker_query,
                            open: picker_open,
                            options: options.read().clone(),
                            enabled: true,
                            vim: false,
                            choose: EventHandler::new(move |option: PickerOption| {
                                destination.set(option.title)
                            }),
                        }),
                ),
        )
        .child(
            label()
                .text(message.read().clone())
                .font_size(t::small())
                .color(p.muted),
        );
    let manuscript = rect().width(Size::fill()).horizontal().content(Content::Flex).spacing(t::SPACE_XL)
        .child(rect().width(Size::flex(1.)).spacing(t::SPACE_MD)
            .child(label().text("Relay Desk").font_size(t::title()).font_weight(FontWeight::MEDIUM))
            .child(rect().width(Size::fill()).spacing(t::SPACE_XS)
                .child(label().text("Mara").font_size(t::small()).color(p.muted))
            .child(t::prose().width(Size::fill()).child(label()
                .text("If you're here about the transmitter, take a number. If you're here about the smoke, take a bucket.")
                .line_height(1.5))))
            .child(rect().width(Size::fill()).spacing(t::SPACE_XS)
                .child(label().text("Reply 1").font_size(t::small()).color(p.muted))
            .child(t::prose().child(label().text("Anything I can do to help?").line_height(1.5))))
            .child(Button::new().flat().on_press(move |_| picker_open.set(true)).child("Missing Courier →")))
        .child(rect().width(Size::px(320.)).spacing(t::SPACE_MD)
            .child(label().text("Graph cards").font_size(t::small()).color(p.muted))
            .child(rect().width(Size::fill()).height(Size::px(156.)).child(BeatCard {
                title: "Relay Desk".into(), speaker: "Mara".into(),
                excerpt: "If you're here about the transmitter, take a number. If you're here about the smoke, take a bucket.".into(),
                annotations: String::new(), selected: true, active: false, zoom: 1.,
            }))
            .child(rect().width(Size::fill()).height(Size::px(156.)).child(BeatCard {
                title: "Missing Courier".into(), speaker: "Mara".into(),
                excerpt: "Our courier is two days late. Everyone keeps asking about the battery she was carrying.".into(),
                annotations: String::new(), selected: false, active: false, zoom: 1.,
            })));
    t::interface()
        .expanded()
        .background(p.canvas)
        .color(p.ink)
        .child(
            ScrollView::new()
                .width(Size::fill())
                .height(Size::fill())
                .child(
                    rect()
                        .width(Size::fill())
                        .padding(t::SPACE_XL)
                        .spacing(t::SPACE_XL)
                        .child(header)
                        .child(
                            rect()
                                .width(Size::fill())
                                .padding(t::PANEL_PADDING)
                                .corner_radius(t::DIALOG_RADIUS)
                                .background(p.surface)
                                .child(controls),
                        )
                        .child(
                            rect()
                                .width(Size::fill())
                                .padding(t::PANEL_PADDING)
                                .corner_radius(t::DIALOG_RADIUS)
                                .background(p.surface)
                                .child(manuscript),
                        ),
                ),
        )
        .into_element()
}
