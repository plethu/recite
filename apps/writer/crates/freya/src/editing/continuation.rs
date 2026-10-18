//! Context changes apply prose on the worker before running their UI continuation.
use super::Writer;
use freya::prelude::*;
use recite_writer_model::{EditOperation, View};

pub(crate) struct Presentation {
    pane: super::Pane,
    view: recite_config::WriterView,
    localising: bool,
    settings: bool,
    commands: Option<crate::commands::SearchMode>,
    modal: bool,
}
impl Presentation {
    /// Palette actions begin after their own dialog has unmounted.
    pub(crate) fn after_palette_close(mut self) -> Self {
        self.commands = None;
        self.modal = false;
        self
    }
    pub(crate) fn capture(writer: Writer) -> Self {
        Self {
            pane: *writer.pane.peek(),
            view: *writer.layout.view.peek(),
            localising: writer.localisation.peek().active,
            settings: *writer.settings_open.peek(),
            commands: *writer.command_search.mode.peek(),
            modal: writer.localisation.peek().modal_open() || crate::design::modal_open(),
        }
    }
    pub(crate) fn is_current(&self, writer: Writer) -> bool {
        self.pane == *writer.pane.peek()
            && self.view == *writer.layout.view.peek()
            && self.localising == writer.localisation.peek().active
            && self.settings == *writer.settings_open.peek()
            && self.commands == *writer.command_search.mode.peek()
            && self.modal
                == (writer.localisation.peek().modal_open() || crate::design::modal_open())
    }
}
impl Writer {
    pub(crate) fn after_prose(self, action: impl FnOnce(Writer) -> Result<(), String> + 'static) {
        let mut message = self.message;
        if let Err(error) = self.work.require_idle() {
            message.error(error);
            return;
        }
        self.buffers.harvest();
        let applies = self
            .buffers
            .model
            .peek()
            .as_ref()
            .is_ok_and(|m| matches!(m.view(), View::Passage(_)) && m.has_draft());
        if !applies {
            if let Err(error) = action(self) {
                message.error(error);
            }
            return;
        }
        let origin = Presentation::capture(self);
        let mut action = Some(action);
        let completed = EventHandler::new(move |result: Result<(), String>| {
            let result = result.and_then(|()| {
                if !origin.is_current(self) || self.buffers.model.peek().as_ref().is_ok_and(|m| m.has_draft()) {
                    return Err("The draft or view changed while the passage was preparing. Your edits remain open; retry the action.".into());
                }
                action.take().ok_or_else(|| "The editing action already completed.".to_string())?(self)
            });
            if let Err(error) = result {
                message.error(error);
            }
        });
        if let Err(error) = crate::editor_work::edit_then(self, EditOperation::Apply, completed) {
            message.error(error);
        }
    }
}
