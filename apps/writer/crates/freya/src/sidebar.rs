//! A stable navigation rail with an animated scene outline and fixed settings footer.
use crate::{controls, editing::Writer, palette};
use freya::{
    animation::{AnimNum, Ease, OnChange, use_animation_with_dependencies},
    prelude::*,
};
use std::{cell::Cell, rc::Rc};

pub(super) fn render(
    mut writer: Writer,
    mut navigation_visible: State<bool>,
    _dark: State<bool>,
    theme: State<Theme>,
    scenes: Element,
) -> Element {
    let mut outline = use_state(|| false);
    let visible = *navigation_visible.read();
    let reduced = writer.preferences.read().config.writer.reduced_motion;
    let current = use_hook(|| Rc::new(Cell::new(if visible { 224. } else { 40. })));
    let previous = current.clone();
    let animation =
        use_animation_with_dependencies(&(visible, reduced), move |c, (visible, reduced)| {
            c.on_change(OnChange::Rerun);
            AnimNum::new(previous.get(), if *visible { 224. } else { 40. })
                .time(if *reduced { 0 } else { 180 })
                .ease(Ease::InOut)
        });
    let width = if reduced {
        if visible { 224. } else { 40. }
    } else {
        animation.get().value()
    };
    current.set(width);
    let colors = theme.read().colors.clone();
    let mut body = rect()
        .width(Size::fill())
        .padding((4., 12.))
        .spacing(4.)
        .a11y_role(AccessibilityRole::Group)
        .a11y_alt("Scenes and beats")
        .child(
            label()
                .text("Scenes")
                .font_size(12.)
                .color(colors.text_secondary),
        )
        .child(scenes);
    body = body.child(
        Button::new()
            .flat()
            .compact()
            .on_press(move |_| {
                let next = !*outline.peek();
                outline.set(next);
            })
            .child(if *outline.read() {
                "▾ Beats in this scene"
            } else {
                "▸ Beats in this scene"
            }),
    );
    if *outline.read()
        && let Ok(session) = writer.buffers.model.read().as_ref()
        && let Ok(blocks) = session.document().script()
    {
        let links = recite_writer_model::scene_links(&blocks);
        let mut beats = rect()
            .width(Size::fill())
            .padding((0., 0., 0., 12.))
            .a11y_role(AccessibilityRole::Group)
            .a11y_alt(format!(
                "{} · {} beats",
                palette::display_name(session.document().key().as_str()),
                blocks.len()
            ));
        for block in blocks {
            let selected = writer.selection.read().as_ref() == Some(&block.id);
            let end = links
                .iter()
                .any(|l| l.origin == block.id && l.destination == "END");
            let caption = format!(
                "{}{}{}",
                if block.is_default { "Start · " } else { "" },
                palette::display_name(&block.id),
                if end { " · end" } else { "" }
            );
            beats = beats.child(controls::navigation_row(
                caption,
                selected,
                writer.dark,
                move |_| {
                    writer.selection.set(Some(block.id.clone()));
                    writer.map_focus.request_focus();
                },
            ));
        }
        body = body.child(beats);
    }
    rect()
        .a11y_id(writer.sidebar_focus)
        .a11y_focusable(true)
        .a11y_role(AccessibilityRole::Navigation)
        .a11y_alt("Scene navigation")
        .width(Size::px(width))
        .height(Size::fill())
        .content(Content::Flex)
        .overflow(Overflow::Clip)
        .child(
            rect()
                .height(Size::px(48.))
                .width(Size::fill())
                .maybe_child(visible.then(|| {
                    rect()
                        .padding((8., 12.))
                        .child(label().text("recite.").font_size(22.))
                }))
                .child(
                    rect()
                        .position(Position::new_absolute().right(4.).top(8.))
                        .child(controls::IconButton::new(
                            if visible {
                                "Hide scenes"
                            } else {
                                "Show scenes"
                            },
                            controls::Icon::Sidebar,
                            move || {
                                let next = !*navigation_visible.peek();
                                navigation_visible.set(next);
                            },
                        )),
                ),
        )
        .child(
            rect()
                .height(Size::flex(1.))
                .width(Size::fill())
                .maybe_child(visible.then(|| {
                    ScrollView::new()
                        .width(Size::fill())
                        .height(Size::fill())
                        .child(body)
                })),
        )
        .child(
            rect()
                .height(Size::px(48.))
                .width(Size::fill())
                .padding((8., 4.))
                .child(controls::IconButton::new(
                    "Settings",
                    controls::Icon::Settings,
                    move || writer.settings_open.set(true),
                )),
        )
        .into_element()
}
