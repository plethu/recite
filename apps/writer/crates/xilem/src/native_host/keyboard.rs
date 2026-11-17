mod bindings;
mod document_menu;
pub(crate) mod focus;
pub(crate) mod galley;
mod held;
mod history;
mod in_view;
mod keys;
use keys::{Keystroke, Stroke, key, physical};
mod manuscript;
mod panes;
mod picker;
mod project;
mod radio;
mod reading;
pub(crate) mod tab;
mod vim;
use crate::app::keyboard::{Action, Context, Invocation, KeyboardContext, Modal, TAG};
use crate::app::picker::Search;
use recite_config::WriterCommand;
use recite_writer_session::commands::keybindings::{self, Input, Key, Modifiers, PhysicalKey};
use xilem::winit::keyboard;
use xilem::{
    masonry::{
        app::RenderRoot,
        core::{Widget, WidgetId},
        widgets::TextInput,
    },
    winit::{
        event::{ElementState, Ime, KeyEvent, WindowEvent},
        keyboard::{ModifiersState, NamedKey},
    },
};

#[derive(Default)]
enum Composition {
    #[default]
    Idle,
    Preedit {
        focus: Option<WidgetId>,
        _guard: Option<crate::attention::Preedit>,
    },
}
#[derive(Default)]
pub(crate) struct Keyboard {
    modifiers: ModifiersState,
    composition: Composition,
    focus: focus::Focus,
    attention: panes::Attention,
    vim: vim::Vim,
    held: held::HeldKeys,
    manuscript: manuscript::Fields,
    document_menu: document_menu::Focus,
    tab: tab::Reveal,
    radio: radio::Arrows,
    galley: galley::Galley,
    reading: reading::Position,
    in_view: in_view::InView,
}
impl Keyboard {
    pub fn reset(&mut self) {
        self.modifiers = ModifiersState::default();
        self.composition = Composition::Idle;
        self.vim = vim::Vim::default();
        self.held = held::HeldKeys::default();
        self.tab = tab::Reveal::default();
        self.radio = radio::Arrows::default();
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
        // Termination ends the lease even if stale field authority rejects the text itself.
        if matches!(event, WindowEvent::Ime(Ime::Commit(_) | Ime::Disabled))
            || matches!(event, WindowEvent::Ime(Ime::Preedit(text, _)) if text.is_empty())
        {
            self.composition = Composition::Idle;
        }
        if manuscript::blocks_input(root, event) {
            return true;
        }
        match event {
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::Ime(Ime::Preedit(text, _)) => {
                self.composition = if text.is_empty() {
                    Composition::Idle
                } else {
                    let guard = manuscript::hold_preedit(root);
                    Composition::Preedit {
                        focus: root.focused_widget(),
                        _guard: guard,
                    }
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
        if event.state == ElementState::Pressed
            && self.modifiers.is_empty()
            && self.radio.request(root, event.logical_key)
        {
            self.held.press(event.physical_key);
            return true;
        }
        let Some(context) = root
            .get_widget_with_tag(TAG)
            .and_then(|widget| widget.get_prop::<KeyboardContext>().0.clone())
        else {
            return false;
        };
        let enter = *event.logical_key == keyboard::Key::Named(NamedKey::Enter);
        let kind = self.focus.kind(root, context.generation);
        let action = bindings::mounted(
            &context,
            &kind,
            event.logical_key,
            self.normalized_modifiers(),
        );
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
                renames: focus::renames(root),
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
            self.tab.request(
                root.focused_widget(),
                self.modifiers.contains(ModifiersState::SHIFT),
            );
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
        if matches!(self.composition, Composition::Preedit { focus, .. } if focus != focused) {
            self.composition = Composition::Idle;
        }
        if !matches!(self.composition, Composition::Idle) {
            return false;
        }
        let Some(action) =
            bindings::resolve(context, &kind, stroke.input, stroke.enter, stroke.action)
        else {
            return false;
        };
        // Rename beat on a sidebar row or map card renames that beat there.
        let action = match (action, stroke.renames) {
            (Action::Command(WriterCommand::Rename), Some(target)) => Action::RenameBeat(target),
            (action, _) => action,
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
        self.radio.synchronize(root);
        let context = root
            .get_widget_with_tag(TAG)
            .and_then(|widget| widget.get_prop::<KeyboardContext>().0.clone());
        if let Some(context) = context {
            self.reading.synchronize(root);
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
            self.reading.capture(root);
            self.in_view.synchronize(
                root,
                context.editing.as_ref().filter(|_| context.writer_active),
            );
            self.galley.synchronize(root);
            self.focus.hold_modal(root, &context);
            self.focus.settle(root);
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

#[cfg(test)]
mod tests;
