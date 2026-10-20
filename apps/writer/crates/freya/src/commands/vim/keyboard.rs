//! Navigation defaults and aliases dispatch through the shared command registry.
use super::*;
use recite_writer_session::commands::vim::{self, Action, Chord};
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
        if let Some(Action::Command(command)) = vim::navigation(chord(event), true) {
            command.run(writer);
        }
        event.stop_propagation();
        event.prevent_default();
        return;
    }
    if crate::commands::shortcut(event).is_some() {
        return;
    }
    match vim::navigation(chord(event), false) {
        Some(Action::PanePrefix) => {
            writer.vim.pending.set(Some(
                *Platform::get()
                    .current_window()
                    .focused_accessibility_id
                    .peek(),
            ));
        }
        Some(Action::Command(command)) => command.run(writer),
        None => return,
    }
    event.stop_propagation();
    event.prevent_default();
}
fn chord(event: &KeyboardEventData) -> Chord<'_> {
    match &event.key {
        Key::Character(key) if event.modifiers == Modifiers::CONTROL => Chord::Control(key),
        Key::Character(key) if (event.modifiers - Modifiers::SHIFT).is_empty() => Chord::Plain(key),
        _ => Chord::Other,
    }
}

/// Exact aliases reuse command validation and persistence; this is not an Ex parser.
pub(in crate::commands) use recite_writer_session::commands::vim_alias as alias;
