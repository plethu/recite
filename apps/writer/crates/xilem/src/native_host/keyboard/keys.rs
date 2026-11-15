//! Winit's keys in the terms of Writer's keybindings, borrowed from the window event.
use super::*;

pub(super) fn key(key: &keyboard::Key) -> Key<'_> {
    match key {
        keyboard::Key::Character(value) => Key::Character(value),
        keyboard::Key::Named(named) => match named {
            NamedKey::F1 => Key::Function(1),
            NamedKey::F2 => Key::Function(2),
            NamedKey::F3 => Key::Function(3),
            NamedKey::F4 => Key::Function(4),
            NamedKey::F5 => Key::Function(5),
            NamedKey::F6 => Key::Function(6),
            NamedKey::F7 => Key::Function(7),
            NamedKey::F8 => Key::Function(8),
            NamedKey::F9 => Key::Function(9),
            NamedKey::F10 => Key::Function(10),
            NamedKey::F11 => Key::Function(11),
            NamedKey::F12 => Key::Function(12),
            _ => Key::Other,
        },
        _ => Key::Other,
    }
}
pub(super) fn physical(key: keyboard::PhysicalKey) -> PhysicalKey {
    use xilem::winit::keyboard::{KeyCode, PhysicalKey as NativePhysical};
    match key {
        NativePhysical::Code(KeyCode::Digit0) => PhysicalKey::Digit(0),
        NativePhysical::Code(KeyCode::Digit1) => PhysicalKey::Digit(1),
        NativePhysical::Code(KeyCode::Digit2) => PhysicalKey::Digit(2),
        NativePhysical::Code(KeyCode::Digit3) => PhysicalKey::Digit(3),
        NativePhysical::Code(KeyCode::Digit4) => PhysicalKey::Digit(4),
        NativePhysical::Code(KeyCode::Digit5) => PhysicalKey::Digit(5),
        NativePhysical::Code(KeyCode::Digit6) => PhysicalKey::Digit(6),
        NativePhysical::Code(KeyCode::Digit7) => PhysicalKey::Digit(7),
        NativePhysical::Code(KeyCode::Digit8) => PhysicalKey::Digit(8),
        NativePhysical::Code(KeyCode::Digit9) => PhysicalKey::Digit(9),
        NativePhysical::Code(KeyCode::Comma) => PhysicalKey::Comma,
        _ => PhysicalKey::Other,
    }
}

pub(super) struct Stroke<'a> {
    pub(super) input: Input<'a>,
    pub(super) enter: bool,
    pub(super) action: Option<Action>,
    pub(super) first_press: bool,
}

/// The window event keeps ownership of text and platform data while policy borrows its keys.
#[derive(Clone, Copy)]
pub(super) struct Keystroke<'a> {
    pub(super) logical_key: &'a keyboard::Key,
    pub(super) physical_key: keyboard::PhysicalKey,
    pub(super) state: ElementState,
    pub(super) repeat: bool,
}
impl<'a> From<&'a KeyEvent> for Keystroke<'a> {
    fn from(event: &'a KeyEvent) -> Self {
        Self {
            logical_key: &event.logical_key,
            physical_key: event.physical_key,
            state: event.state,
            repeat: event.repeat,
        }
    }
}
