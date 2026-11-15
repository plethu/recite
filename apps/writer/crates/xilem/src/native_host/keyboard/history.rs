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
