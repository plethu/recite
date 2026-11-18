//! Reading aids do not navigate, own another draft, or outlive the application.
pub use crate::{Writer, navigation::Target};
use recite_core::DocumentKey;
use recite_writer_session::reading::ReferenceSnapshot;
use std::collections::BTreeSet;
#[cfg(test)]
mod tests;

#[derive(Default)]
pub struct Context {
    pub pinned: Option<Pinned>,
    expanded: BTreeSet<Destination>,
}

pub struct Pinned {
    pub snapshot: ReferenceSnapshot,
    pub collapsed: bool,
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Destination {
    document: DocumentKey,
    beat: String,
    target: String,
}
impl Writer {
    pub fn pin_reference(&mut self) {
        self.navigate(Target::PinReference);
    }
    /// Whether the pinned reference is the selected beat of the open document.
    pub fn reference_pinned(&self) -> bool {
        self.reading_context.pinned.as_ref().is_some_and(|pinned| {
            pinned.snapshot.document() == self.session.model().document().key()
                && self.reading.selected.as_deref() == Some(pinned.snapshot.block().id.as_str())
        })
    }
    /// The beat heading's pin toggle: unpin the selected beat when it is the reference, otherwise
    /// pin it (replacing any other reference).
    pub fn toggle_reference(&mut self) {
        if self.reference_pinned() {
            self.reading_context.pinned = None;
        } else {
            self.pin_reference();
        }
    }
    pub fn publish_reference(&mut self) -> Result<(), String> {
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
    pub fn destination_expanded(&self, target: &str) -> bool {
        let Some(beat) = &self.reading.selected else {
            return false;
        };
        self.reading_context.expanded.contains(&Destination {
            document: self.session.model().document().key().clone(),
            beat: beat.clone(),
            target: target.to_owned(),
        })
    }
    pub fn toggle_destination_preview(&mut self, target: String) {
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
