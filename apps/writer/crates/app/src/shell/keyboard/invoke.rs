//! Applying a captured invocation: stale ones are refused, current ones reach the shell.
use super::*;

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
            Action::DismissModal(Modal::Settings) => self.close_settings(),
            Action::DismissModal(Modal::PreviewInputs) => {
                if let Some(writer) = &mut self.writer {
                    writer.close_trial_inputs();
                }
            }
            Action::DismissModal(Modal::Rename) => {
                if let Some(writer) = &mut self.writer {
                    writer.cancel_inline_rename();
                }
            }
            Action::DismissModal(Modal::CloseDecision) => {
                if let Some(writer) = &mut self.writer {
                    writer.keep_editing();
                }
            }
            Action::ChoosePreview(index) => {
                if let Some(writer) = &mut self.writer {
                    writer.choose_preview(index);
                }
            }
            Action::PaneHint(pending) => self.pane_hint = pending,
            Action::PlacePicker { x, y } => {
                if let Some(picker) = &mut self.picker {
                    picker.placement = Some((x, y));
                }
            }
            Action::StepBeat { forward } => {
                if let Some(writer) = &mut self.writer {
                    writer.step_beat(forward);
                }
            }
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
