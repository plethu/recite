//! Runnable native specimen of production controls. No project or preferences are written.
mod panels;
use super::{
    Button, PickerOption, ReducedMotion, SearchField, SearchPicker, Segments, palette, tokens as t,
};
use freya::prelude::*;
use std::sync::Arc;

fn tier(name: &str, control: Button) -> Rect {
    rect()
        .spacing(t::SPACE_XS)
        .child(t::caps(name))
        .child(control)
}

pub(crate) fn app() -> Element {
    let mut dark = use_state(|| true);
    let mut monochrome = use_state(|| false);
    let mut theme = use_init_theme(|| palette::theme_with(true, false));
    let mut reduced = use_state(|| false);
    use_provide_context(|| ReducedMotion(reduced));
    let query = use_state(String::new);
    let active = use_state(|| None);
    let picker_query = use_state(String::new);
    let picker_open = use_state(|| false);
    let mut destination = use_state(|| "Missing Courier".to_owned());
    let mut view = use_state(|| 0);
    let mut row = use_state(|| 1);
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
            SvgViewer::new(Bytes::from_static(include_bytes!(
                "../../../../../../assets/identity/recite-wordmark.svg"
            )))
            .width(Size::px(28. * t::WORDMARK_ASPECT))
            .height(Size::px(28.))
            .fill(p.wordmark)
            .color(p.teal)
            .a11y_alt("Recite"),
        )
        .child(t::meta("native component specimen"))
        .child(rect().width(Size::flex(1.)))
        .child(
            Button::new()
                .flat()
                .checkable(*reduced.read())
                .highlighted(*reduced.read())
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
                    theme.set(palette::theme_with(next, *monochrome.peek()));
                })
                .child(if *dark.read() {
                    "Light appearance"
                } else {
                    "Dark appearance"
                }),
        )
        .child(
            Button::new()
                .toggle(*monochrome.read())
                .on_press(move |_| {
                    let next = !*monochrome.peek();
                    monochrome.set(next);
                    theme.set(palette::theme_with(*dark.peek(), next));
                })
                .child("Monochrome"),
        );
    let controls = rect()
        .width(Size::fill())
        .spacing(t::SPACE_MD)
        .child(t::caps("Actions and navigation"))
        .child(
            rect()
                .horizontal()
                .cross_align(Alignment::End)
                .spacing(t::SPACE_LG)
                .child(tier(
                    "Primary",
                    Button::new()
                        .filled()
                        .on_press(move |_| message.set("Primary action activated.".into()))
                        .child("Try scene"),
                ))
                .child(tier(
                    "Stamp",
                    Button::new()
                        .stamp()
                        .on_press(move |_| message.set("Secondary action activated.".into()))
                        .child("Add beat"),
                ))
                .child(tier(
                    "Quiet",
                    Button::new()
                        .flat()
                        .on_press(move |_| message.set("Quiet action activated.".into()))
                        .child("Arrange automatically"),
                ))
                .child(tier(
                    "Disabled",
                    Button::new().stamp().enabled(false).child("Save changes"),
                ))
                .child(Segments {
                    shortcuts: None,
                    name: "Writing view".into(),
                    labels: ["Map".into(), "Source".into()],
                    ids: [use_a11y(), use_a11y()],
                    selected: *view.read(),
                    vim: false,
                    width: Size::px(180.),
                    change: EventHandler::new(move |index| view.set(index)),
                })
                .child(t::meta("⌘↵ try · ⌘B add")),
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
                            insert_request: None,
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
    let rows = ["Relay Hub", "Start · Relay Desk", "Missing Courier"];
    let mut list = rect()
        .width(Size::px(240.))
        .spacing(t::SPACE_XS)
        .child(t::caps("Scenes"));
    for (index, caption) in rows.into_iter().enumerate() {
        list = list.child(
            Button::new()
                .flat()
                .row_gutter()
                .toggle(index == *row.read())
                .width(Size::fill())
                .on_press(move |_| row.set(index))
                .child(rect().width(Size::fill()).child(label().text(caption))),
        );
    }
    let lists = rect()
        .width(Size::fill())
        .horizontal()
        .spacing(t::SPACE_XL)
        .child(list)
        .child(
            rect()
                .width(Size::flex(1.))
                .spacing(t::SPACE_SM)
                .child(t::caps("Meta"))
                .child(t::meta(
                    "relay_hub.recite  :: relay_desk  default  4 replies",
                ))
                .child(t::meta("@7701ceab59d2adfa057a"))
                .child(
                    rect()
                        .width(Size::fill())
                        .height(Size::px(1.))
                        .background(p.rule),
                )
                .child(
                    label()
                        .text("Hairlines are ink at 14%; selection is the second-ink plate.")
                        .font_size(t::small())
                        .color(p.muted),
                ),
        );
    let print = panels::elevation(&p);
    let manuscript = panels::manuscript(picker_open);
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
                                .child(lists),
                        )
                        .child(
                            rect()
                                .width(Size::fill())
                                .padding(t::PANEL_PADDING)
                                .corner_radius(t::DIALOG_RADIUS)
                                .background(p.surface)
                                .child(print),
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
