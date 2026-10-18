//! Dispatch only after the invoking palette releases its modal registration.
use super::{Dialog, Item, Writer};
use crate::editing::Presentation;
use freya::prelude::*;
use recite_writer_model::RecoveredDraft;

struct Pending {
    item: Item,
    presentation: Presentation,
    scene: Option<String>,
    draft: Option<RecoveredDraft>,
}
#[derive(Clone, Copy, PartialEq)]
pub(super) struct Handoff {
    pending: State<Option<Pending>>,
    mounted: State<bool>,
}
pub(super) fn use_handoff(writer: Writer) -> Handoff {
    let mut handoff = Handoff {
        pending: use_state(|| None),
        mounted: use_state(|| false),
    };
    use_after_side_effect(move || {
        if handoff.pending.read().is_none() {
            return;
        }
        if writer.command_search.mode.read().is_some() {
            handoff.pending.set(None);
        } else if !*handoff.mounted.read() {
            handoff.dispatch(writer);
        }
    });
    handoff
}
impl Handoff {
    pub(super) fn submit(mut self, item: Item, writer: Writer) {
        writer.buffers.harvest();
        let pending = Pending {
            item,
            presentation: Presentation::capture(writer).after_palette_close(),
            scene: writer.scene_key(),
            draft: writer
                .buffers
                .model
                .peek()
                .as_ref()
                .ok()
                .map(|m| m.recovery()),
        };
        writer.command_search.close();
        self.pending.set(Some(pending));
    }
    pub(super) fn dismiss(mut self, writer: Writer) {
        self.pending.set(None);
        writer.command_search.close();
    }
    fn dispatch(&mut self, writer: Writer) {
        let Some(pending) = self.pending.write().take() else {
            return;
        };
        writer.buffers.harvest();
        let current = writer
            .buffers
            .model
            .peek()
            .as_ref()
            .ok()
            .map(|m| m.recovery());
        let same_draft = match (&pending.draft, &current) {
            (Some(before), Some(now)) => {
                std::sync::Arc::ptr_eq(&before.source_snapshot(), &now.source_snapshot())
                    && before == now
            }
            (None, None) => true,
            _ => false,
        };
        if same_draft
            && pending.scene == writer.scene_key()
            && pending.presentation.is_current(writer)
            && pending.item.enabled(writer)
        {
            pending.item.run(writer);
        }
    }
    pub(super) fn dialog(self, dialog: Dialog) -> impl IntoElement {
        MountedDialog {
            dialog,
            handoff: self,
        }
    }
}
#[derive(Clone, PartialEq)]
struct MountedDialog {
    dialog: Dialog,
    handoff: Handoff,
}
impl Component for MountedDialog {
    fn render(&self) -> impl IntoElement {
        let mut handoff = self.handoff;
        use_hook(move || handoff.mounted.set(true));
        use_drop(move || handoff.mounted.set(false));
        self.dialog.clone()
    }
}

#[cfg(test)]
mod tests;
