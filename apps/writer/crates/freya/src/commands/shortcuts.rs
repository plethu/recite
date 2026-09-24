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
    if binding.is_empty() {
        return false;
    }
    let mut parts = binding.rsplit('+');
    let key = parts.next().unwrap_or_default();
    let mut modifiers = Modifiers::empty();
    for part in parts {
        modifiers |= match part {
            "Primary" => primary(),
            "Alt" => Modifiers::ALT,
            "Shift" => Modifiers::SHIFT,
            _ => return false,
        };
    }
    if event.modifiers != modifiers {
        return false;
    }
    // Shifted digit/punctuation chords still name the base key printed in settings.
    if event.modifiers.contains(Modifiers::SHIFT)
        && (key == "," || key.bytes().all(|c| c.is_ascii_digit()))
    {
        let code = if key == "," {
            "Comma".to_owned()
        } else {
            format!("Digit{key}")
        };
        return code.parse::<Code>().is_ok_and(|code| code == event.code);
    }
    match &event.key {
        Key::Character(value) => value.eq_ignore_ascii_case(key),
        Key::Named(named) => key.parse::<NamedKey>().is_ok_and(|key| key == *named),
    }
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
    let window = *Platform::get().root_size.read();
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
