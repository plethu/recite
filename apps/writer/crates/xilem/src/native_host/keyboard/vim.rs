//! A pane prefix is bound to its native focus and keyboard scope; every consumed release is owned.
use super::*;
use crate::app::keyboard::{Action, CommandGeneration};
use recite_writer_session::commands::vim::{self, Chord};
use xilem::winit::event::KeyEvent;

struct Prefix {
    generation: CommandGeneration,
    focus: Option<WidgetId>,
}
#[derive(Default)]
pub(super) struct Vim {
    pending: Option<Prefix>,
    held: held::HeldKeys,
}
impl Vim {
    pub fn release(&mut self, physical: keyboard::PhysicalKey, state: ElementState) -> bool {
        self.held.consume(physical, state)
    }
    pub fn synchronize(&mut self, context: &Context, focus: Option<WidgetId>) {
        if self.pending.as_ref().is_some_and(|pending| {
            pending.generation != context.generation
                || pending.focus != focus
                || !context.writer_active
                || context.picker_open
                || context.keymap != recite_config::Keymap::Vim
        }) {
            self.cancel(context);
        }
    }
    pub fn cancel(&mut self, context: &Context) {
        if self.pending.take().is_some() {
            let _ = context.proxy.message(Invocation {
                action: Action::PaneHint(false),
                generation: context.generation,
            });
        }
    }
    pub fn event(
        &mut self,
        context: &Context,
        focus: Option<WidgetId>,
        kind: focus::Kind,
        event: &KeyEvent,
        modifiers: ModifiersState,
    ) -> bool {
        self.synchronize(context, focus);
        if !context.writer_active
            || context.picker_open
            || context.keymap != recite_config::Keymap::Vim
            || event.state != ElementState::Pressed
            || event.repeat
        {
            return false;
        }
        if matches!(
            event.logical_key,
            keyboard::Key::Named(
                NamedKey::Control
                    | NamedKey::Shift
                    | NamedKey::Alt
                    | NamedKey::Meta
                    | NamedKey::Super
            )
        ) {
            return false;
        }
        let chord = match &event.logical_key {
            keyboard::Key::Character(value) if modifiers == ModifiersState::CONTROL => {
                Chord::Control(value)
            }
            keyboard::Key::Character(value)
                if (modifiers - ModifiersState::SHIFT).is_empty() && kind == focus::Kind::Other =>
            {
                Chord::Plain(value)
            }
            keyboard::Key::Character(value)
                if self.pending.is_some() && (modifiers - ModifiersState::SHIFT).is_empty() =>
            {
                Chord::Plain(value)
            }
            _ => Chord::Other,
        };
        self.press(context, focus, chord, event.physical_key)
    }
    fn press(
        &mut self,
        context: &Context,
        focus: Option<WidgetId>,
        chord: Chord<'_>,
        physical: keyboard::PhysicalKey,
    ) -> bool {
        let pending = self.pending.is_some();
        let action = vim::navigation(chord, pending);
        if pending {
            self.cancel(context);
        }
        match action {
            Some(vim::Action::PanePrefix) => {
                self.pending = Some(Prefix {
                    generation: context.generation,
                    focus,
                });
                let _ = context.proxy.message(Invocation {
                    action: Action::PaneHint(true),
                    generation: context.generation,
                });
            }
            Some(vim::Action::Command(command)) => {
                let _ = context.proxy.message(Invocation {
                    action: Action::Command(command),
                    generation: context.generation,
                });
            }
            None if !pending => return false,
            None => {}
        }
        self.held.press(physical);
        true
    }
}

#[cfg(test)]
mod tests;
