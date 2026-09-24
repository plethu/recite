//! Translate physical key events into the portable persisted shortcut vocabulary.
use freya::prelude::*;
use recite_config::WriterShortcut;

pub(super) fn primary() -> Modifiers {
    if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    }
}
pub(super) fn primary_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd"
    } else {
        "Ctrl"
    }
}
pub(super) fn binding(
    event: &KeyboardEventData,
    modifiers: Option<Modifiers>,
) -> Option<Result<String, String>> {
    if matches!(
        event.key,
        Key::Named(NamedKey::Control | NamedKey::Meta | NamedKey::Alt | NamedKey::Shift)
    ) {
        return None;
    }
    let modifiers = modifiers.unwrap_or(event.modifiers);
    if !(modifiers - (primary() | Modifiers::ALT | Modifiers::SHIFT)).is_empty() {
        return Some(Err(
            "That system modifier cannot be used for a shortcut.".into()
        ));
    }
    let key = match &event.key {
        Key::Character(value) => {
            // Shift may change the logical character; retain the base digit/comma.
            match event.code {
                Code::Digit0 => "0".into(),
                Code::Digit1 => "1".into(),
                Code::Digit2 => "2".into(),
                Code::Digit3 => "3".into(),
                Code::Digit4 => "4".into(),
                Code::Digit5 => "5".into(),
                Code::Digit6 => "6".into(),
                Code::Digit7 => "7".into(),
                Code::Digit8 => "8".into(),
                Code::Digit9 => "9".into(),
                Code::Comma => ",".into(),
                _ => value.to_uppercase(),
            }
        }
        Key::Named(named) => named.to_string(),
    };
    let mut parts = Vec::new();
    if modifiers.contains(primary()) {
        parts.push("Primary");
    }
    if modifiers.contains(Modifiers::ALT) {
        parts.push("Alt");
    }
    if modifiers.contains(Modifiers::SHIFT) {
        parts.push("Shift");
    }
    parts.push(&key);
    Some(WriterShortcut::try_from(parts.join("+")).map(|b| b.as_str().to_owned()).map_err(|_| {
        "Use Ctrl/Cmd with a letter, number or comma, or a function key other than F6. Editing and system shortcuts cannot be reassigned.".into()
    }))
}
