//! Anchored secondary passage actions, with owned keyboard focus and dismissal.
use crate::{design::Button, editing::Writer};
use freya::prelude::*;
use recite_writer_model::Workbench;

pub(super) fn render(writer: Writer, mut details: State<bool>) -> Element {
    let mut open = use_state(|| false);
    let trigger = use_a11y();
    let items = [use_a11y(), use_a11y()];
    use_after_side_effect(move || {
        if *open.read() {
            items[0].request_focus();
        }
    });
    let mut anchor = rect().child(action(trigger, "Passage actions ▾", false, move || {
        let next = !*open.peek();
        open.set(next);
    }));
    if *open.read() {
        anchor = anchor.child(
            rect()
                .position(Position::new_absolute().top(34.).right(0.))
                .layer(Layer::Overlay)
                .on_key_down(move |event: Event<KeyboardEventData>| {
                    let focus = *Platform::get().focused_accessibility_id.peek();
                    let current = usize::from(focus == items[1]);
                    let vim =
                        writer.preferences.peek().config.ui.keymap == recite_config::Keymap::Vim;
                    let next = if crate::design::keyboard::list_step(&event, vim).is_some() {
                        Some(1 - current)
                    } else {
                        match event.key {
                            Key::Named(NamedKey::Home) => Some(0),
                            Key::Named(NamedKey::End) => Some(1),
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
                        .child(action(items[1], "Add choice", true, move || {
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
        .child(button.child(label().width(Size::fill()).text(caption)))
        .into_element()
}
