//! Anchored secondary passage actions, with owned keyboard focus and dismissal.
use crate::{editing::Writer, palette};
use freya::prelude::*;
use recite_bakeoff_authoring::Workbench;

pub(super) fn render(writer: Writer, mut details: State<bool>) -> Element {
    let mut open = use_state(|| false);
    let trigger = use_a11y();
    let items = [use_a11y(), use_a11y()];
    use_after_side_effect(move || {
        if *open.read() {
            items[0].request_focus();
        }
    });
    let mut anchor = rect().child(action(
        trigger,
        "Passage actions ▾",
        writer.dark,
        false,
        move || {
            let next = !*open.peek();
            open.set(next);
        },
    ));
    if *open.read() {
        anchor = anchor.child(
            rect()
                .position(Position::new_absolute().top(34.).right(0.))
                .layer(Layer::Overlay)
                .on_key_down(move |event: Event<KeyboardEventData>| {
                    let focus = *Platform::get().focused_accessibility_id.peek();
                    let current = usize::from(focus == items[1]);
                    let next = match event.key {
                        Key::Named(NamedKey::ArrowDown | NamedKey::ArrowUp) => Some(1 - current),
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
                        .child(action(
                            items[0],
                            "Line / choice details",
                            writer.dark,
                            true,
                            move || {
                                let next = !*details.peek();
                                details.set(next);
                                open.set(false);
                                trigger.request_focus();
                            },
                        ))
                        .child(action(
                            items[1],
                            "Add choice",
                            writer.dark,
                            true,
                            move || {
                                writer.perform(Workbench::add_choice);
                                open.set(false);
                                trigger.request_focus();
                            },
                        )),
                ),
        );
    }
    anchor.into_element()
}

fn action(
    id: AccessibilityId,
    caption: &'static str,
    dark: bool,
    menu_item: bool,
    action: impl FnMut() + 'static,
) -> Element {
    let mut action = action;
    let action = EventHandler::new(move |()| action());
    rect()
        .a11y_id(id)
        .a11y_focusable(true)
        .a11y_role(if menu_item {
            AccessibilityRole::MenuItem
        } else {
            AccessibilityRole::Button
        })
        .cursor(CursorIcon::Pointer)
        .padding(Gaps::new(7., 12., 7., 12.))
        .min_width(Size::px(if menu_item { 190. } else { 0. }))
        .corner_radius(4.)
        .background(if id.is_focused() {
            palette::selection(dark)
        } else {
            Color::TRANSPARENT
        })
        .border(Border::new().width(1.).fill(if id.is_focused() {
            palette::accent(dark)
        } else {
            Color::TRANSPARENT
        }))
        .on_press(move |event: Event<PressEventData>| {
            event.stop_propagation();
            id.request_focus();
            action.call(());
        })
        .child(label().text(caption))
        .into_element()
}
