//! Shared native shortcut and list-navigation policy.
use freya::prelude::*;

pub(crate) fn submit_key(event: &KeyboardEventData) -> bool {
    event.key == Key::Named(NamedKey::Enter) && event.modifiers == primary_modifier()
}

pub(crate) fn primary_modifier() -> Modifiers {
    if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    }
}

pub(crate) fn submit_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "⌘ ↵"
    } else {
        "Ctrl ↵"
    }
}

/// A logical colon may require Shift on the current keyboard layout.
/// Only navigation owners may use this; text entry must keep literal colons.
pub(crate) fn vim_commands_key(event: &KeyboardEventData) -> bool {
    matches!(&event.key, Key::Character(key) if key == ":")
        && (event.modifiers - Modifiers::SHIFT).is_empty()
}

pub(crate) fn vim_workspace_key(event: &KeyboardEventData) -> bool {
    if vim_commands_key(event) {
        return true;
    }
    if event.modifiers == Modifiers::CONTROL {
        matches!(&event.key, Key::Character(k) if ["o", "i", "w"].iter().any(|key| k.eq_ignore_ascii_case(key)))
    } else {
        (event.modifiers - Modifiers::SHIFT).is_empty()
            && matches!(&event.key, Key::Character(k) if [":", "/", "n", "N"].contains(&k.as_str()))
    }
}

/// Call with `normal = false` while a text field is accepting text.
pub(crate) fn list_step(event: &KeyboardEventData, normal: bool) -> Option<isize> {
    if !event.modifiers.is_empty() {
        return None;
    }
    match &event.key {
        Key::Named(NamedKey::ArrowDown) => Some(1),
        Key::Named(NamedKey::ArrowUp) => Some(-1),
        Key::Character(key) if normal && key == "j" => Some(1),
        Key::Character(key) if normal && key == "k" => Some(-1),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
