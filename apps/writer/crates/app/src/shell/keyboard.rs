//! Keyboard command freshness, context and dispatch; the frontend only delivers invocations.
use super::*;
pub use crate::attention::{DocumentMenu, EditingTarget, FieldRestoration, PaneMotion};
pub use crate::localisation::TranslationTarget;
use recite_config::{WriterCommand, WriterShortcuts};

mod scope;
pub use scope::Scope;

/// Freshness of commands captured from one mounted keyboard context.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CommandGeneration(u64);
impl CommandGeneration {
    fn advance(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}
/// Picker opening reads current state and does not own an earlier analysis snapshot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PickerOpeningGeneration(u64);
impl PickerOpeningGeneration {
    fn advance(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}
/// What the native keyboard host may do now; `P` delivers invocations back to the shell.
pub struct Context<P> {
    pub shortcuts: WriterShortcuts,
    pub generation: CommandGeneration,
    pub picker_opening_generation: PickerOpeningGeneration,
    pub writer_active: bool,
    pub screen: Option<crate::workspaces::Screen>,
    pub picker_open: bool,
    pub picker_instance: Option<picker::PickerInstance>,
    pub project_form_ready: bool,
    pub document_menu: DocumentMenu,
    pub attention: crate::attention::Attention,
    pub editing: Option<EditingTarget>,
    pub field_restoration: Option<FieldRestoration>,
    pub keymap: recite_config::Keymap,
    pub proxy: P,
}
pub struct Invocation {
    pub action: Action,
    pub generation: CommandGeneration,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    OpenPicker {
        search: picker::Search,
        generation: PickerOpeningGeneration,
    },
    Command(WriterCommand),
    SaveTranslation(TranslationTarget),
    SubmitPicker,
    SubmitProject,
    PaneHint(bool),
    CyclePane {
        forward: bool,
    },
    FieldFocus(Option<EditingTarget>),
    DismissDocumentMenu,
}
impl std::fmt::Debug for Invocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandInvocation")
            .field("action", &self.action)
            .finish_non_exhaustive()
    }
}
impl Scope {
    /// The command context a host mounted for this scope exposes to native input.
    pub fn context<P>(
        &self,
        shortcuts: &WriterShortcuts,
        generations: Generations,
        proxy: P,
    ) -> Context<P> {
        Context {
            shortcuts: shortcuts.clone(),
            generation: generations.command,
            picker_opening_generation: generations.picker_opening,
            writer_active: self.writer.as_ref().is_some_and(|writer| {
                writer.writing && writer.screen == crate::workspaces::Screen::Write
            }) && !self.welcome
                && !self.settings,
            picker_open: self.picker.is_some(),
            picker_instance: self
                .picker
                .as_ref()
                .map(|(instance, _, _)| instance.clone()),
            project_form_ready: self.project_form.as_ref().is_some_and(|form| form.ready),
            document_menu: self
                .writer
                .as_ref()
                .map_or(DocumentMenu::Closed, |writer| writer.document_menu),
            screen: self
                .writer
                .as_ref()
                .filter(|writer| writer.writing && !self.welcome && !self.settings)
                .map(|writer| writer.screen),
            attention: self
                .writer
                .as_ref()
                .map(|writer| writer.attention.clone())
                .unwrap_or_default(),
            editing: self
                .writer
                .as_ref()
                .filter(|writer| {
                    writer.writing
                        && writer.screen == crate::workspaces::Screen::Write
                        && !self.welcome
                        && !self.settings
                        && self.picker.is_none()
                })
                .and_then(|writer| writer.editing.clone()),
            keymap: self.keymap,
            field_restoration: self
                .writer
                .as_ref()
                .and_then(|writer| writer.field_restoration.clone()),
            proxy,
        }
    }
}

/// Command and picker-opening freshness captured by one mounted keyboard host.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Generations {
    pub command: CommandGeneration,
    pub picker_opening: PickerOpeningGeneration,
}
impl Generations {
    /// Retire invocations captured before the scope or shortcuts changed.
    pub fn update(
        &mut self,
        previous: (&Scope, &WriterShortcuts),
        current: (&Scope, &WriterShortcuts),
    ) {
        if !current.0.same_command_scope(previous.0) || current.1 != previous.1 {
            self.command.advance();
        }
        if !current.0.same_picker_opening_scope(previous.0) || current.1 != previous.1 {
            self.picker_opening.advance();
        }
    }
}

impl Shell {
    /// Apply an invocation captured under `captured`; returns false when it is stale.
    pub fn invoke(
        &mut self,
        captured: &Scope,
        shortcuts: &WriterShortcuts,
        generations: Generations,
        invocation: Invocation,
    ) -> bool {
        let current = Scope::capture(self);
        let current_scope = match &invocation.action {
            Action::OpenPicker { generation, .. } => {
                *generation == generations.picker_opening
                    && captured.same_picker_opening_scope(&current)
            }
            _ => {
                invocation.generation == generations.command
                    && captured.same_command_scope(&current)
            }
        };
        if !current_scope || shortcuts != &self.preferences.config.writer.shortcuts {
            return false;
        }
        match invocation.action {
            Action::OpenPicker { search, .. } => {
                if let Some(writer) = &mut self.writer {
                    writer.document_menu = DocumentMenu::Closed;
                }
                self.open_picker(search);
            }
            Action::FieldFocus(target) => {
                if let Some(writer) = &mut self.writer {
                    writer.native_field_focus(target);
                }
            }
            Action::Command(command) => self.run_command(command),
            Action::SaveTranslation(target) => {
                if let Some(writer) = &mut self.writer {
                    writer.save_focused_translation(&target);
                }
            }
            Action::SubmitPicker => self.submit_picker(),
            Action::SubmitProject => self.open_selected(),
            Action::DismissDocumentMenu => {
                if let Some(writer) = &mut self.writer {
                    writer.document_menu = DocumentMenu::Dismissed;
                }
            }
            Action::PaneHint(pending) => self.pane_hint = pending,
            Action::CyclePane { forward } => {
                if let Some(writer) = &mut self.writer {
                    writer.focus_pane(if forward {
                        PaneMotion::Next
                    } else {
                        PaneMotion::Previous
                    });
                }
            }
        }
        true
    }
}
