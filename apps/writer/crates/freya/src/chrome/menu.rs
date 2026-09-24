//! Less frequent workspace actions remain reachable at every text size.
use crate::commands::CommandExt;
use crate::{
    commands::Command,
    design::{Button, tokens as t},
    editing::Writer,
    messages::{MsgId, text},
};
use freya::prelude::*;

pub(super) fn render(writer: Writer, writing: bool) -> Element {
    let mut open = use_state(|| false);
    let trigger = use_a11y();
    let ids: [AccessibilityId; 8] = std::array::from_fn(|_| use_a11y());
    let commands: Vec<_> = if writing {
        vec![
            Command::Split,
            Command::Focus,
            Command::GoTo,
            Command::Localise,
            Command::Save,
            Command::SaveAll,
            Command::Settings,
        ]
    } else {
        vec![
            Command::Script,
            Command::GoTo,
            Command::Save,
            Command::SaveAll,
            Command::Settings,
        ]
    };
    use_after_side_effect(move || {
        if *open.read() {
            ids[0].request_focus();
        }
    });
    let mut anchor = rect().child(
        Button::new()
            .a11y_id(trigger)
            .named(text(MsgId::WriterWorkspaceMenu))
            .expanded(*open.read())
            .on_press(move |_| {
                let next = !*open.peek();
                open.set(next);
            })
            .child(text(MsgId::WriterWorkspaceMenu))
            .child(crate::controls::Icon::ChevronDown.render()),
    );
    if *open.read() {
        let enabled_ids: Vec<_> = commands
            .iter()
            .enumerate()
            .filter(|(_, command)| command.enabled(writer))
            .map(|(index, _)| ids[index])
            .collect();
        let count = enabled_ids.len();
        let mut menu = Menu::new()
            .on_close(move |_| open.set(false))
            .on_escape(move |_| {
                open.set(false);
                trigger.request_focus();
            });
        for (index, command) in commands.into_iter().enumerate() {
            menu = menu.child(
                Button::new()
                    .menu_item()
                    .width(Size::fill())
                    .a11y_id(ids[index])
                    .named(command.label(writer))
                    .enabled(command.enabled(writer))
                    .on_press(move |_| {
                        open.set(false);
                        trigger.request_focus();
                        if command == Command::Settings {
                            crate::settings::open(writer, trigger);
                        } else {
                            command.run(writer);
                        }
                    })
                    .child(label().width(Size::fill()).text(command.label(writer))),
            );
        }
        anchor = anchor.child(
            rect()
                .position(Position::new_absolute().right(0.).top(t::control_height()))
                .layer(Layer::Overlay)
                .width(Size::px(280. * t::ui_scale()))
                .on_key_down(move |event: Event<KeyboardEventData>| {
                    let focused = *Platform::get().focused_accessibility_id.peek();
                    let current = enabled_ids
                        .iter()
                        .position(|id| *id == focused)
                        .unwrap_or(0);
                    let next = match event.key {
                        Key::Named(NamedKey::ArrowDown) => (current + 1) % count,
                        Key::Named(NamedKey::ArrowUp) => (current + count - 1) % count,
                        Key::Named(NamedKey::Home) => 0,
                        Key::Named(NamedKey::End) => count - 1,
                        Key::Named(NamedKey::Tab) => {
                            event.prevent_default();
                            event.stop_propagation();
                            open.set(false);
                            trigger.request_focus();
                            return;
                        }
                        _ => return,
                    };
                    event.prevent_default();
                    event.stop_propagation();
                    enabled_ids[next].request_focus();
                })
                .child(menu),
        );
    }
    anchor.into_element()
}
