//! Bindings mounted for the moment, tried before the configured shortcuts: the open picker, the
//! project form, the document menu, history, beat stepping and leaving focus mode.
use super::*;
#[cfg(test)]
mod tests;

/// The action a stroke asks for: a mounted binding's, Primary+Enter applying a manuscript draft,
/// F6 cycling panes, or a configured shortcut, with the pickers opened at the current generation.
pub(super) fn resolve(
    context: &Context,
    kind: &focus::Kind,
    input: Input<'_>,
    enter: bool,
    mounted: Option<Action>,
) -> Option<Action> {
    let apply = context.writer_active
        && *kind == focus::Kind::Manuscript
        && enter
        && input.modifiers
            == Modifiers {
                primary: true,
                ..Modifiers::default()
            };
    let action = if let Some(action) = mounted {
        action
    } else if apply {
        Action::Command(WriterCommand::Apply)
    } else if context.writer_active
        && !context.picker_open
        && input.key == Key::Function(6)
        && !input.modifiers.primary
        && !input.modifiers.alt
        && !input.modifiers.other
    {
        Action::CyclePane {
            forward: !input.modifiers.shift,
        }
    } else {
        keybindings::command(&context.shortcuts, input).map(|command| match (kind, command) {
            (focus::Kind::Translation(target), WriterCommand::Save) => {
                Action::SaveTranslation(target.clone())
            }
            _ => Action::Command(command),
        })?
    };
    Some(match action {
        Action::Command(WriterCommand::Commands) => Action::OpenPicker {
            search: Search::Commands,
            generation: context.picker_opening_generation,
        },
        Action::Command(WriterCommand::GoTo | WriterCommand::Find) => Action::OpenPicker {
            search: Search::GoTo,
            generation: context.picker_opening_generation,
        },
        other => other,
    })
}

/// The first mounted binding that takes `key` with `modifiers`.
pub(super) fn mounted(
    context: &Context,
    kind: &focus::Kind,
    key: &keyboard::Key,
    modifiers: Modifiers,
) -> Option<Action> {
    picker::action(context, kind.clone(), key, modifiers)
        .or_else(|| project::submit(context, kind, key, modifiers))
        .or_else(|| document_menu::escape(context, key, modifiers))
        .or_else(|| history::key(context, key, modifiers))
        .or_else(|| beat_key(context, key, modifiers))
        .or_else(|| focus_key(context, key, modifiers))
}

/// Primary+Up and Primary+Down move between the scene's beats in Script.
fn beat_key(context: &Context, key: &keyboard::Key, modifiers: Modifiers) -> Option<Action> {
    if context.screen != Some(crate::workspaces::Screen::Write)
        || context.picker_open
        || modifiers
            != (Modifiers {
                primary: true,
                ..Modifiers::default()
            })
    {
        return None;
    }
    match key {
        keyboard::Key::Named(NamedKey::ArrowDown) => Some(Action::StepBeat { forward: true }),
        keyboard::Key::Named(NamedKey::ArrowUp) => Some(Action::StepBeat { forward: false }),
        _ => None,
    }
}
/// Escape leaves focus mode when nothing nearer, such as a picker, takes it.
fn focus_key(context: &Context, key: &keyboard::Key, modifiers: Modifiers) -> Option<Action> {
    (context.attention.is_focused()
        && !context.picker_open
        && modifiers == Modifiers::default()
        && *key == keyboard::Key::Named(NamedKey::Escape))
    .then_some(Action::Command(WriterCommand::Focus))
}
