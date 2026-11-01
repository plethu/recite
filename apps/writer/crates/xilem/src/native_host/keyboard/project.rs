//! Submit project fields through the same composition and held-release owner as commands.
use super::*;

pub(super) fn submit(
    context: &Context,
    kind: &focus::Kind,
    key: &keyboard::Key,
    modifiers: Modifiers,
) -> Option<Action> {
    (context.project_form_ready
        && !context.picker_open
        && *kind == focus::Kind::ProjectForm
        && *key == keyboard::Key::Named(NamedKey::Enter)
        && modifiers == Modifiers::default())
    .then_some(Action::SubmitProject)
}
