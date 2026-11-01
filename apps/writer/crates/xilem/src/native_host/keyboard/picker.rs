//! Picker submission belongs to the host, including the release after focus restoration.
use super::*;

pub(super) fn action(
    context: &Context,
    kind: focus::Kind,
    key: &keyboard::Key,
    modifiers: Modifiers,
) -> Option<Action> {
    if !context.picker_open || modifiers != Modifiers::default() {
        return None;
    }
    match key {
        keyboard::Key::Named(NamedKey::Enter)
            if matches!(kind, focus::Kind::Query | focus::Kind::ActiveResult) =>
        {
            Some(Action::SubmitPicker)
        }
        keyboard::Key::Named(NamedKey::Escape) => Some(Action::Command(WriterCommand::Close)),
        keyboard::Key::Named(NamedKey::ArrowDown) if kind == focus::Kind::Query => {
            Some(Action::Command(WriterCommand::NextMatch))
        }
        keyboard::Key::Named(NamedKey::ArrowUp) if kind == focus::Kind::Query => {
            Some(Action::Command(WriterCommand::PreviousMatch))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;
