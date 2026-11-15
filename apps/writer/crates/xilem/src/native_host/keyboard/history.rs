//! Standard desktop history gestures use the same checked command path as the toolbar.
use super::*;
use xilem::winit::event::MouseButton;

pub(super) fn key(context: &Context, key: &keyboard::Key, modifiers: Modifiers) -> Option<Action> {
    if context.screen.is_none()
        || context.picker_open
        || modifiers
            != (Modifiers {
                alt: true,
                ..Modifiers::default()
            })
    {
        return None;
    }
    let command = match key {
        keyboard::Key::Named(NamedKey::ArrowLeft) => WriterCommand::Back,
        keyboard::Key::Named(NamedKey::ArrowRight) => WriterCommand::Forward,
        _ => return None,
    };
    Some(Action::Command(command))
}
/// Primary+Up and Primary+Down move between the scene's beats in Script.
pub(super) fn beat_key(
    context: &Context,
    key: &keyboard::Key,
    modifiers: Modifiers,
) -> Option<Action> {
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
pub(super) fn focus_key(
    context: &Context,
    key: &keyboard::Key,
    modifiers: Modifiers,
) -> Option<Action> {
    (context.attention.is_focused()
        && !context.picker_open
        && modifiers == Modifiers::default()
        && *key == keyboard::Key::Named(NamedKey::Escape))
    .then_some(Action::Command(WriterCommand::Focus))
}
impl Keyboard {
    pub(super) fn history_mouse(
        &mut self,
        root: &RenderRoot,
        button: MouseButton,
        state: ElementState,
    ) -> bool {
        let command = match button {
            MouseButton::Back => WriterCommand::Back,
            MouseButton::Forward => WriterCommand::Forward,
            _ => return false,
        };
        let Some(context) = root
            .get_widget_with_tag(TAG)
            .and_then(|widget| widget.get_prop::<KeyboardContext>().0.clone())
        else {
            return false;
        };
        if context.screen.is_none()
            || context.picker_open
            || !matches!(self.composition, Composition::Idle)
        {
            return false;
        }
        if state == ElementState::Pressed {
            self.vim.cancel(&context);
            let _ = context.proxy.message(Invocation {
                action: Action::Command(command),
                generation: context.generation,
            });
        }
        true
    }
}

#[cfg(test)]
mod tests;
