mod document_menu;
mod focus;
mod held;
mod history;
mod manuscript;
mod panes;
mod picker;
mod project;
mod tab;
mod vim;
use crate::app::keyboard::{Action, Context, Invocation, KeyboardContext, TAG};
use recite_config::WriterCommand;
use recite_writer_session::commands::keybindings::{self, Input, Key, Modifiers, PhysicalKey};
use xilem::winit::keyboard;
use xilem::{
    masonry::{app::RenderRoot, core::WidgetId, widgets::TextInput},
    winit::{
        event::{ElementState, Ime, KeyEvent, WindowEvent},
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
    held: held::HeldKeys,
    manuscript: manuscript::Fields,
    document_menu: document_menu::Focus,
    tab: tab::Reveal,
}
impl Keyboard {
    pub fn reset(&mut self) {
        self.modifiers = ModifiersState::default();
        self.composition = Composition::Idle;
        self.vim = vim::Vim::default();
        self.held = held::HeldKeys::default();
        self.tab = tab::Reveal::default();
    }
    pub fn blur(&mut self, root: &mut RenderRoot) {
        if let Some(context) = root
            .get_widget_with_tag(TAG)
            .and_then(|widget| widget.get_prop::<KeyboardContext>().0.clone())
        {
            self.vim.cancel(&context);
            self.manuscript.clear(root, &context);
        }
        self.reset();
    }
    pub fn event(&mut self, root: &mut RenderRoot, event: &WindowEvent) -> bool {
        if manuscript::blocks_input(root, event) {
            return true;
        }
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
            WindowEvent::MouseInput { button, state, .. } => {
                return self.history_mouse(root, *button, *state);
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => return self.key_event(root, Keystroke::from(event), *is_synthetic),
            _ => {}
        }
        false
    }
    fn key_event(&mut self, root: &RenderRoot, event: Keystroke<'_>, is_synthetic: bool) -> bool {
        if self.held.consume(event.physical_key, event.state)
            || self.vim.release(event.physical_key, event.state)
        {
            return true;
        }
        if manuscript::unadmitted(root)
            && event.state == ElementState::Pressed
            && !matches!(
                event.logical_key,
                keyboard::Key::Named(NamedKey::Tab | NamedKey::Escape)
            )
        {
            self.held.press(event.physical_key);
            return true;
        }
        if is_synthetic {
            return false;
        }
        let Some(context) = root
            .get_widget_with_tag(TAG)
            .and_then(|widget| widget.get_prop::<KeyboardContext>().0.clone())
        else {
            return false;
        };
        let enter = *event.logical_key == keyboard::Key::Named(NamedKey::Enter);
        let kind = self.focus.kind(root, context.generation);
        let action = picker::action(
            &context,
            kind.clone(),
            event.logical_key,
            self.normalized_modifiers(),
        )
        .or_else(|| {
            project::submit(
                &context,
                &kind,
                event.logical_key,
                self.normalized_modifiers(),
            )
        })
        .or_else(|| document_menu::escape(&context, event.logical_key, self.normalized_modifiers()))
        .or_else(|| history::key(&context, event.logical_key, self.normalized_modifiers()));
        let consumed = self.dispatch(
            &context,
            root.focused_widget(),
            kind.clone(),
            Stroke {
                input: Input {
                    key: key(event.logical_key),
                    physical: physical(event.physical_key),
                    modifiers: self.normalized_modifiers(),
                },
                enter,
                action,
                first_press: event.state == ElementState::Pressed && !event.repeat,
            },
        );
        if consumed && event.state == ElementState::Pressed {
            self.held.press(event.physical_key);
        }
        if consumed || !matches!(self.composition, Composition::Idle) {
            self.vim.cancel(&context);
            return consumed;
        }
        let consumed = self
            .vim
            .event(&context, root.focused_widget(), kind, event, self.modifiers);
        if !consumed
            && event.state == ElementState::Pressed
            && *event.logical_key == keyboard::Key::Named(NamedKey::Tab)
            && !self
                .modifiers
                .intersects(ModifiersState::CONTROL | ModifiersState::ALT | ModifiersState::SUPER)
        {
            self.tab.request(root.focused_widget());
        }
        consumed
    }
    fn dispatch(
        &mut self,
        context: &Context,
        focused: Option<WidgetId>,
        kind: focus::Kind,
        stroke: Stroke<'_>,
    ) -> bool {
        if matches!(self.composition, Composition::Preedit(focus) if focus != focused) {
            self.composition = Composition::Idle;
        }
        if !matches!(self.composition, Composition::Idle) {
            return false;
        }
        let apply = context.writer_active
            && kind == focus::Kind::Manuscript
            && stroke.enter
            && stroke.input.modifiers
                == Modifiers {
                    primary: true,
                    ..Modifiers::default()
                };
        let action = if let Some(action) = stroke.action {
            Some(action)
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
            keybindings::command(&context.shortcuts, stroke.input).map(|command| {
                match (&kind, command) {
                    (focus::Kind::Translation(target), WriterCommand::Save) => {
                        Action::SaveTranslation(target.clone())
                    }
                    _ => Action::Command(command),
                }
            })
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
        self.tab.synchronize(root);
        let context = root
            .get_widget_with_tag(TAG)
            .and_then(|widget| widget.get_prop::<KeyboardContext>().0.clone());
        if let Some(context) = context {
            if matches!(self.composition, Composition::Idle) {
                self.vim.synchronize(&context, root.focused_widget());
            } else {
                self.vim.cancel(&context);
            }
            self.focus.synchronize(root, &context);
            self.attention.synchronize(root, &context);
            if !context.picker_open {
                self.focus.synchronize_manuscript(
                    root,
                    context.editing.as_ref().filter(|_| context.writer_active),
                    context.attention.edit_request(),
                );
            }
            self.manuscript
                .synchronize(root, &context, &self.composition);
            self.document_menu.synchronize(root, &context);
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
    action: Option<Action>,
    first_press: bool,
}

/// The window event keeps ownership of text and platform data while policy borrows its keys.
#[derive(Clone, Copy)]
struct Keystroke<'a> {
    logical_key: &'a keyboard::Key,
    physical_key: keyboard::PhysicalKey,
    state: ElementState,
    repeat: bool,
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

#[cfg(test)]
mod tests;
