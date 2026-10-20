//! Live validation admission also protects callbacks queued before a field was disabled.
use super::{Writer, focus};
use freya::prelude::*;
use recite_writer_model::View;

pub(crate) fn validate(writer: Writer, target: &View, id: AccessibilityId, value: InputValidator) {
    if writer.project_edit_blocked() {
        value.set_valid(false);
        return;
    }
    let mut buffers = writer.buffers;
    let selected = buffers
        .model
        .peek()
        .as_ref()
        .is_ok_and(|model| model.view() == target);
    if !selected && let View::Passage(passage) = target {
        focus::select(writer, passage.clone(), id);
    }
    if let Ok(session) = buffers.model.write().as_mut() {
        if session.view() == target {
            let draft = value.text().clone();
            session.set_draft(draft.clone());
            buffers.prose.set_if_modified(draft);
        } else {
            value.set_valid(false);
        }
    }
}
