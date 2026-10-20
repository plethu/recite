//! Shared dispatch and discovery read the same persisted bindings.
use super::Command;
use crate::presentation::Typography;
use freya::prelude::*;

#[derive(Clone, Copy)]
pub(crate) struct Hints(pub State<Modifiers>);

pub(super) fn primary() -> Modifiers {
    if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    }
}

pub(crate) fn shortcut(event: &KeyboardEventData) -> Option<Command> {
    let preferences = try_consume_context::<Typography>();
    let defaults = recite_config::WriterShortcuts::default();
    let config = preferences.as_ref().map(|p| p.0.peek());
    let bindings = config
        .as_ref()
        .map_or(&defaults, |p| &p.config.writer.shortcuts);
    Command::ALL
        .iter()
        .copied()
        .find(|command| matches_binding(bindings.binding(*command), event))
}

fn matches_binding(binding: &str, event: &KeyboardEventData) -> bool {
    use recite_writer_session::commands::keybindings::{
        self, Input, Key as ShortcutKey, Modifiers as ShortcutModifiers, PhysicalKey,
    };
    let key = match &event.key {
        Key::Character(value) => ShortcutKey::Character(value),
        Key::Named(named) => match named {
            NamedKey::F1 => ShortcutKey::Function(1),
            NamedKey::F2 => ShortcutKey::Function(2),
            NamedKey::F3 => ShortcutKey::Function(3),
            NamedKey::F4 => ShortcutKey::Function(4),
            NamedKey::F5 => ShortcutKey::Function(5),
            NamedKey::F6 => ShortcutKey::Function(6),
            NamedKey::F7 => ShortcutKey::Function(7),
            NamedKey::F8 => ShortcutKey::Function(8),
            NamedKey::F9 => ShortcutKey::Function(9),
            NamedKey::F10 => ShortcutKey::Function(10),
            NamedKey::F11 => ShortcutKey::Function(11),
            NamedKey::F12 => ShortcutKey::Function(12),
            _ => ShortcutKey::Other,
        },
    };
    let physical = match event.code {
        Code::Digit0 => PhysicalKey::Digit(0),
        Code::Digit1 => PhysicalKey::Digit(1),
        Code::Digit2 => PhysicalKey::Digit(2),
        Code::Digit3 => PhysicalKey::Digit(3),
        Code::Digit4 => PhysicalKey::Digit(4),
        Code::Digit5 => PhysicalKey::Digit(5),
        Code::Digit6 => PhysicalKey::Digit(6),
        Code::Digit7 => PhysicalKey::Digit(7),
        Code::Digit8 => PhysicalKey::Digit(8),
        Code::Digit9 => PhysicalKey::Digit(9),
        Code::Comma => PhysicalKey::Comma,
        _ => PhysicalKey::Other,
    };
    keybindings::matches(
        binding,
        Input {
            key,
            physical,
            modifiers: ShortcutModifiers {
                primary: event.modifiers.contains(primary()),
                alt: event.modifiers.contains(Modifiers::ALT),
                shift: event.modifiers.contains(Modifiers::SHIFT),
                other: !(event.modifiers & !(primary() | Modifiers::ALT | Modifiers::SHIFT))
                    .is_empty(),
            },
        },
    )
}

pub(crate) fn hint(binding: &str, area: Option<Area>) -> Option<Element> {
    let area = area?;
    let hints = try_consume_context::<Hints>()?;
    let preferences = try_consume_context::<Typography>()?;
    let preferences = preferences.0.read();
    if binding.is_empty() {
        return None;
    }
    let held = *hints.0.read();
    let relevant = ((binding.contains("Ctrl+") || binding.contains("Cmd+"))
        && held.contains(primary()))
        || (binding.contains("Alt+") && held.contains(Modifiers::ALT));
    if !preferences.config.writer.shortcut_hints && !relevant {
        return None;
    }
    let p = crate::design::tokens::colors();
    let size = crate::design::tokens::small();
    let width = binding.len() as f32 * size * 0.65 + 10.;
    let window = *Platform::get().current_window().root_size.read();
    let left = area.min_x().min((window.width - width - 4.).max(0.));
    let top = if area.max_y() + size + 8. > window.height {
        area.min_y() - size - 8.
    } else {
        area.max_y() - 2.
    };
    Some(
        rect()
            .position(Position::new_global().top(top).left(left))
            .width(Size::px(width))
            .layer(Layer::Overlay)
            .padding((1., 4.))
            .corner_radius(3.)
            .background(p.inset)
            .border(Border::new().width(1.).fill(p.boundary))
            .color(p.ink)
            .child(
                label()
                    .text(binding.to_owned())
                    .font_family("monospace")
                    .max_lines(1)
                    .font_size(crate::design::tokens::small()),
            )
            .into_element(),
    )
}

#[cfg(test)]
mod tests;
