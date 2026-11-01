//! Match validated portable bindings against normalized native keyboard input.
use recite_config::{WriterCommand, WriterShortcuts};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub primary: bool,
    pub alt: bool,
    pub shift: bool,
    pub other: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key<'a> {
    Character(&'a str),
    Function(u8),
    Other,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PhysicalKey {
    Digit(u8),
    Comma,
    #[default]
    Other,
}
#[derive(Clone, Copy, Debug)]
pub struct Input<'a> {
    pub key: Key<'a>,
    pub physical: PhysicalKey,
    pub modifiers: Modifiers,
}
pub fn command(shortcuts: &WriterShortcuts, input: Input<'_>) -> Option<WriterCommand> {
    WriterCommand::ALL
        .iter()
        .copied()
        .find(|command| matches(shortcuts.binding(*command), input))
}
pub fn matches(binding: &str, input: Input<'_>) -> bool {
    if binding.is_empty() {
        return false;
    }
    let mut parts = binding.rsplit('+');
    let key = parts.next().unwrap_or_default();
    let mut modifiers = Modifiers::default();
    for modifier in parts {
        match modifier {
            "Primary" => modifiers.primary = true,
            "Alt" => modifiers.alt = true,
            "Shift" => modifiers.shift = true,
            _ => return false,
        }
    }
    if modifiers != input.modifiers {
        return false;
    }
    if modifiers.shift {
        if key == "," {
            return input.physical == PhysicalKey::Comma;
        }
        if key.len() == 1 && key.as_bytes()[0].is_ascii_digit() {
            return input.physical == PhysicalKey::Digit(key.as_bytes()[0] - b'0');
        }
    }
    match input.key {
        Key::Character(value) => value.eq_ignore_ascii_case(key),
        Key::Function(index) => {
            key.strip_prefix('F').and_then(|key| key.parse::<u8>().ok()) == Some(index)
        }
        Key::Other => false,
    }
}

#[cfg(test)]
mod tests;
