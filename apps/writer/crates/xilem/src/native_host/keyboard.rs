mod focus;
mod panes;
mod vim;
use crate::app::keyboard::{Action, Context, Invocation, KeyboardContext, TAG};
use recite_config::WriterCommand;
use recite_writer_session::commands::keybindings::{self, Input, Key, Modifiers, PhysicalKey};
use xilem::winit::keyboard;
use xilem::{
    masonry::{app::RenderRoot, core::WidgetId, widgets::TextInput},
    winit::{
        event::{ElementState, Ime, WindowEvent},
        keyboard::{ModifiersState, NamedKey},
    },
};

#[derive(Default)]
enum Composition {
    #[default]
    Idle,
    Preedit(Option<WidgetId>),
}
#[derive(Default)]
pub(super) struct Keyboard {
    modifiers: ModifiersState,
    composition: Composition,
    focus: focus::Focus,
    attention: panes::Attention,
    vim: vim::Vim,
}
impl Keyboard {
    pub fn reset(&mut self) {
        self.modifiers = ModifiersState::default();
        self.composition = Composition::Idle;
        self.vim = vim::Vim::default();
    }
    pub fn blur(&mut self, root: &RenderRoot) {
        if let Some(context) = root
            .get_widget_with_tag(TAG)
            .and_then(|widget| widget.get_prop::<KeyboardContext>().0.clone())
        {
            self.vim.cancel(&context);
        }
        self.reset();
    }
    pub fn event(&mut self, root: &RenderRoot, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::Ime(Ime::Preedit(text, _)) => {
                self.composition = if text.is_empty() {
                    Composition::Idle
                } else {
                    Composition::Preedit(root.focused_widget())
                };
            }
            WindowEvent::Ime(Ime::Commit(_) | Ime::Disabled) => {
                self.composition = Composition::Idle
            }
            WindowEvent::Focused(false) => self.blur(root),
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => {
                let Some(context) = root
                    .get_widget_with_tag(TAG)
                    .and_then(|widget| widget.get_prop::<KeyboardContext>().0.clone())
                else {
                    return false;
                };
                let enter = event.logical_key == keyboard::Key::Named(NamedKey::Enter);
                let kind = self.focus.kind(root);
                let picker = if context.picker_open
                    && self.normalized_modifiers() == Modifiers::default()
                {
                    match event.logical_key {
                        keyboard::Key::Named(NamedKey::Escape) => Some(WriterCommand::Close),
                        keyboard::Key::Named(NamedKey::ArrowDown) if kind == focus::Kind::Query => {
                            Some(WriterCommand::NextMatch)
                        }
                        keyboard::Key::Named(NamedKey::ArrowUp) if kind == focus::Kind::Query => {
                            Some(WriterCommand::PreviousMatch)
                        }
                        _ => None,
                    }
                } else {
                    None
                };
                let manuscript = enter && kind == focus::Kind::Manuscript;
                if self.vim.release(event.physical_key, event.state) {
                    return true;
                }
                let consumed = self.dispatch(
                    &context,
                    root.focused_widget(),
                    manuscript,
                    Stroke {
                        input: Input {
                            key: key(&event.logical_key),
                            physical: physical(event.physical_key),
                            modifiers: self.normalized_modifiers(),
                        },
                        enter,
                        picker,
                        first_press: event.state == ElementState::Pressed && !event.repeat,
                    },
                );
                if consumed || !matches!(self.composition, Composition::Idle) {
                    self.vim.cancel(&context);
                    return consumed;
                }
                return self.vim.event(
                    &context,
                    root.focused_widget(),
                    kind,
                    event,
                    self.modifiers,
                );
            }
            _ => {}
        }
        false
    }
    fn dispatch(
        &mut self,
        context: &Context,
        focused: Option<WidgetId>,
        manuscript: bool,
        stroke: Stroke<'_>,
    ) -> bool {
        if matches!(self.composition, Composition::Preedit(focus) if focus != focused) {
            self.composition = Composition::Idle;
        }
        if !matches!(self.composition, Composition::Idle) {
            return false;
        }
        let apply = context.writer_active
            && manuscript
            && stroke.enter
            && stroke.input.modifiers
                == Modifiers {
                    primary: true,
                    ..Modifiers::default()
                };
        let action = if stroke.picker.is_some() {
            stroke.picker.map(Action::Command)
        } else if apply {
            Some(Action::Command(WriterCommand::Apply))
        } else if context.writer_active
            && !context.picker_open
            && stroke.input.key == Key::Function(6)
            && !stroke.input.modifiers.primary
            && !stroke.input.modifiers.alt
            && !stroke.input.modifiers.other
        {
            Some(Action::CyclePane {
                forward: !stroke.input.modifiers.shift,
            })
        } else {
            keybindings::command(&context.shortcuts, stroke.input).map(Action::Command)
        };
        let Some(action) = action else {
            return false;
        };
        // Clear the hint before a command can retire this keyboard scope.
        self.vim.cancel(context);
        if stroke.first_press {
            let _ = context.proxy.message(Invocation {
                action,
                generation: context.generation,
            });
        }
        true
    }
    pub fn synchronize(&mut self, root: &mut RenderRoot) {
        let context = root
            .get_widget_with_tag(TAG)
            .and_then(|widget| widget.get_prop::<KeyboardContext>().0.clone());
        if let Some(context) = context {
            if matches!(self.composition, Composition::Idle) {
                self.vim.synchronize(&context, root.focused_widget());
            } else {
                self.vim.cancel(&context);
            }
            self.focus
                .synchronize(root, context.picker_open, context.generation);
            self.attention.synchronize(root, &context);
        }
    }
    fn normalized_modifiers(&self) -> Modifiers {
        Modifiers {
            primary: if cfg!(target_os = "macos") {
                self.modifiers.super_key()
            } else {
                self.modifiers.control_key()
            },
            alt: self.modifiers.alt_key(),
            shift: self.modifiers.shift_key(),
            other: if cfg!(target_os = "macos") {
                self.modifiers.control_key()
            } else {
                self.modifiers.super_key()
            },
        }
    }
}

fn key(key: &keyboard::Key) -> Key<'_> {
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
fn physical(key: keyboard::PhysicalKey) -> PhysicalKey {
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

struct Stroke<'a> {
    input: Input<'a>,
    enter: bool,
    picker: Option<WriterCommand>,
    first_press: bool,
}

#[cfg(test)]
mod tests;
