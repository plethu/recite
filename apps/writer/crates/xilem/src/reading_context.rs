//! Reading aids do not navigate, own another draft, or outlive the application.
use crate::{Writer, navigation::Target};
use recite_core::DocumentKey;
use recite_writer_session::reading::ReferenceSnapshot;
use std::collections::BTreeSet;

#[derive(Default)]
pub(super) struct Context {
    pub pinned: Option<Pinned>,
    expanded: BTreeSet<Destination>,
}

#[path = "reading/context/tests.rs"]
#[cfg(test)]
mod tests;
pub(super) struct Pinned {
    pub snapshot: ReferenceSnapshot,
    pub collapsed: bool,
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Destination {
    document: DocumentKey,
    beat: String,
    target: String,
}
impl Writer {
    pub(super) fn pin_reference(&mut self) {
        self.navigate(Target::PinReference);
    }
    pub(super) fn publish_reference(&mut self) -> Result<(), String> {
        let snapshot = ReferenceSnapshot::capture(self.session.model())
            .map_err(|error| error.to_string())?
            .ok_or("Choose a beat before pinning a reference.")?;
        self.reading_context.pinned = Some(Pinned {
            snapshot,
            collapsed: false,
        });
        self.trial.cancel();
        self.trial.visible = false;
        self.trial.settings.open = false;
        Ok(())
    }
    pub(super) fn destination_expanded(&self, target: &str) -> bool {
        let Some(beat) = &self.reading.selected else {
            return false;
        };
        self.reading_context.expanded.contains(&Destination {
            document: self.session.model().document().key().clone(),
            beat: beat.clone(),
            target: target.to_owned(),
        })
    }
    pub(super) fn toggle_destination_preview(&mut self, target: String) {
        let Some(beat) = &self.reading.selected else {
            return;
        };
        let destination = Destination {
            document: self.session.model().document().key().clone(),
            beat: beat.clone(),
            target,
        };
        if !self.reading_context.expanded.remove(&destination) {
            self.reading_context.expanded.insert(destination);
        }
    }
}
