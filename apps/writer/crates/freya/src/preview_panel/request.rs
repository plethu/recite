//! The requested trial owns its settings; only workspace identity gates receipt.
use super::{Snapshot, prepare::Inputs};
use crate::{editing::Pane, localisation::catalogue::Catalogue};
use recite_writer_model::{PreviewRequest, View, Workbench};

pub(super) struct Request {
    pub source: PreviewRequest,
    pub view: View,
    pub inputs: Inputs,
    pub snapshot: Snapshot,
}
impl Request {
    pub fn is_current(&self, model: &Workbench, pane: Pane, catalogue: Option<&Catalogue>) -> bool {
        pane == Pane::Preview
            && model.view() == &self.view
            && !self.snapshot.stale(catalogue)
            && self.source.is_current(model)
    }
}

#[cfg(test)]
mod tests;
