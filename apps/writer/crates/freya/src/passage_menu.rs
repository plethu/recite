//! Anchored secondary passage actions, with owned keyboard focus and dismissal.
use crate::{design::Button, editing::Writer};
use freya::prelude::*;
use recite_writer_model::{Passage, PassageKind, Workbench};

pub(super) fn render(writer: Writer, mut details: State<bool>, passage: &Passage) -> Element {
    let choice = matches!(passage.kind, PassageKind::Choice { .. });
    let rule_id = passage.id.clone();
    let count = if choice { 3 } else { 2 };
    let mut open = use_state(|| false);
    let trigger = use_a11y();
    let items = [use_a11y(), use_a11y(), use_a11y()];
    use_after_side_effect(move || {
        if *open.read() {
            items[0].request_focus();
        }
    });
    let mut anchor = rect().width(Size::px(148.)).child(action(
        trigger,
        "Passage actions ▾",
        false,
        move || {
            let next = !*open.peek();
            open.set(next);
        },
    ));
    if *open.read() {
        anchor = anchor.child(
            rect()
                .position(Position::new_absolute().top(38.).right(0.))
                .width(Size::px(232.))
                .layer(Layer::Overlay)
                .on_key_down(move |event: Event<KeyboardEventData>| {
                    let focus = *Platform::get().focused_accessibility_id.peek();
                    let current = items.iter().position(|id| *id == focus).unwrap_or(0);
                    let vim =
                        writer.preferences.peek().config.ui.keymap == recite_config::Keymap::Vim;
                    let next = if let Some(step) = crate::design::keyboard::list_step(&event, vim) {
                        Some((current as isize + step).rem_euclid(count as isize) as usize)
                    } else {
                        match event.key {
                            Key::Named(NamedKey::Home) => Some(0),
                            Key::Named(NamedKey::End) => Some(count - 1),
                            Key::Named(NamedKey::Tab) => {
                                event.prevent_default();
                                event.stop_propagation();
                                open.set(false);
                                trigger.request_focus();
                                None
                            }
                            _ => None,
                        }
                    };
                    if let Some(next) = next {
                        event.prevent_default();
                        event.stop_propagation();
                        items[next].request_focus();
                    }
                })
                .child(
                    Menu::new()
                        .on_close(move |_| open.set(false))
                        .on_escape(move |_| {
                            open.set(false);
                            trigger.request_focus();
                        })
                        .child(action(items[0], "Line / choice details", true, move || {
                            let next = !*details.peek();
                            details.set(next);
                            open.set(false);
                            trigger.request_focus();
                        }))
                        .maybe_child(choice.then(|| {
                            action(items[1], "Reply rules", true, move || {
                                let mut writer = writer;
                                if let Err(error) = crate::rules::open(writer, &rule_id) {
                                    writer.message.error(error);
                                }
                                open.set(false);
                            })
                        }))
                        .child(action(items[count - 1], "Add choice", true, move || {
                            writer.perform(Workbench::add_choice);
                            open.set(false);
                            trigger.request_focus();
                        })),
                ),
        );
    }
    anchor.into_element()
}

fn action(
    id: AccessibilityId,
    caption: &'static str,
    menu_item: bool,
    action: impl FnMut() + 'static,
) -> Element {
    let mut action = action;
    let button = Button::new()
        .flat()
        .a11y_id(id)
        .named(caption)
        .on_press(move |_| action());
    let button = if menu_item {
        button.menu_item().width(Size::fill())
    } else {
        button
    };
    rect()
        .min_width(Size::px(if menu_item { 210. } else { 0. }))
        .child(button.child(if menu_item {
            label().width(Size::fill()).text(caption).into_element()
        } else {
            rect()
                .horizontal()
                .cross_align(Alignment::Center)
                .spacing(6.)
                .child(label().text(crate::messages::text(
                    crate::messages::MsgId::WriterGuiPassageActions,
                )))
                .child(crate::controls::Icon::ChevronDown.render())
                .into_element()
        }))
        .into_element()
}
