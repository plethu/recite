//! Navigation defaults and aliases dispatch through the shared command registry.
use super::*;
pub(crate) fn modifier_key(event: &KeyboardEventData) -> bool {
    event.modifiers == Modifiers::CONTROL
        && matches!(&event.key, Key::Character(k) if ["o", "i", "w"].iter().any(|key| k.eq_ignore_ascii_case(key)))
}
pub(in crate::commands) fn modifier_keyboard(
    writer: Writer,
    event: &Event<KeyboardEventData>,
) -> bool {
    if writer.preferences.peek().config.ui.keymap == recite_config::Keymap::Vim
        && modifier_key(event)
    {
        dispatch(writer, event);
        true
    } else {
        false
    }
}
pub(crate) fn keyboard(writer: Writer, event: &Event<KeyboardEventData>) {
    if !modifier_key(event) || writer.vim.pending.peek().is_some() {
        dispatch(writer, event);
    }
}
fn dispatch(mut writer: Writer, event: &Event<KeyboardEventData>) {
    if crate::design::modal_open()
        || writer.preferences.peek().config.ui.keymap != recite_config::Keymap::Vim
        || *writer.settings_open.peek()
        || writer.localisation.peek().modal_open()
        || writer.command_search.mode.peek().is_some()
    {
        writer.vim.cancel();
        return;
    }
    if matches!(
        event.key,
        Key::Named(NamedKey::Control | NamedKey::Shift | NamedKey::Meta | NamedKey::Alt)
    ) {
        return;
    }
    let pending = *writer.vim.pending.peek();
    if let Some(invoker) = pending {
        writer.vim.cancel();
        invoker.request_focus();
        let command = if (event.modifiers - Modifiers::CONTROL).is_empty() {
            match &event.key {
                Key::Character(k) => match k.as_str() {
                    "h" => Some(Command::PaneLeft),
                    "j" => Some(Command::PaneDown),
                    "k" => Some(Command::PaneUp),
                    "l" => Some(Command::PaneRight),
                    _ => None,
                },
                _ => None,
            }
        } else {
            None
        };
        if let Some(command) = command {
            command.run(writer);
        }
        event.stop_propagation();
        event.prevent_default();
        return;
    }
    if crate::commands::shortcut(event).is_some() {
        return;
    }
    let command = if event.modifiers == Modifiers::CONTROL {
        match &event.key {
            Key::Character(k) if k.eq_ignore_ascii_case("o") => Some(Command::Back),
            Key::Character(k) if k.eq_ignore_ascii_case("i") => Some(Command::Forward),
            Key::Character(k) if k.eq_ignore_ascii_case("w") => {
                writer
                    .vim
                    .pending
                    .set(Some(*Platform::get().focused_accessibility_id.peek()));
                event.stop_propagation();
                event.prevent_default();
                return;
            }
            _ => None,
        }
    } else if (event.modifiers - Modifiers::SHIFT).is_empty() {
        match &event.key {
            Key::Character(k) => match k.as_str() {
                ":" => Some(Command::Commands),
                "/" => Some(Command::Find),
                "n" => Some(Command::NextMatch),
                "N" => Some(Command::PreviousMatch),
                _ => None,
            },
            _ => None,
        }
    } else {
        None
    };
    if let Some(command) = command {
        command.run(writer);
        event.stop_propagation();
        event.prevent_default();
    }
}

/// Exact aliases reuse command validation and persistence; this is not an Ex parser.
pub(in crate::commands) fn alias(query: &str) -> Option<Command> {
    match query.trim().trim_start_matches(':') {
        "w" => Some(Command::Save),
        "wa" => Some(Command::SaveAll),
        "q" => Some(Command::Close),
        _ => None,
    }
}
