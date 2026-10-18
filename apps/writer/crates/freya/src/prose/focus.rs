//! One latest focus intent follows prose preparation without queuing stale field switches.
use crate::editing::{Pane, Writer};
use freya::prelude::*;
use recite_writer_model::{EditOperation, View};

pub(crate) struct Request {
    scene: Option<String>,
    passage: String,
    id: AccessibilityId,
    pane: Pane,
}
pub(crate) fn select(mut writer: Writer, passage: String, id: AccessibilityId) {
    let request = {
        let model = writer.buffers.model.peek();
        let Ok(model) = model.as_ref() else {
            return;
        };
        (model.view() != &View::Passage(passage.clone())).then(|| Request {
            scene: writer.scene_key(),
            passage,
            id,
            pane: *writer.pane.peek(),
        })
    };
    writer.work.focus.set(request);
    complete(writer, true);
}
pub(crate) fn complete(mut writer: Writer, successful: bool) {
    if !successful {
        writer.work.focus.set(None);
        return;
    }
    if writer.work.busy() {
        return;
    }
    let Some(request) = writer.work.focus.write().take() else {
        return;
    };
    if !request.id.is_focused() || *writer.pane.peek() != request.pane {
        return;
    }
    if writer.scene_key() != request.scene {
        return;
    }
    writer.buffers.harvest();
    let dirty = writer
        .buffers
        .model
        .peek()
        .as_ref()
        .is_ok_and(|m| matches!(m.view(), View::Passage(_)) && m.has_draft());
    if dirty {
        writer.work.focus.set(Some(request));
        if let Err(error) = crate::editor_work::edit(writer, EditOperation::Apply) {
            writer.work.focus.set(None);
            writer.message.error(error);
        }
    } else if let Err(error) = writer.try_navigate(|m| m.select(View::Passage(request.passage))) {
        writer.message.error(error);
    }
}
