//! Frontend review state wraps the shared source-refresh decision.
use super::extraction;
use crate::{editing::Writer, project::ProjectFiles};
use freya::prelude::State;
use recite_writer_session::localisation::catalogue::{RefreshApplied, RefreshPreview};
mod detail;
pub(super) mod screen;

pub(crate) struct Preview {
    source: RefreshPreview,
    applied: bool,
}
impl Preview {
    pub fn prepare(writer: Writer, files: State<Option<ProjectFiles>>) -> Result<Self, String> {
        let template = extraction::template(writer, files)?;
        let state = writer.localisation.peek();
        let catalogue = state.catalogue.as_ref().ok_or("Open a catalogue first.")?;
        Ok(Self {
            source: RefreshPreview::prepare(catalogue, template)?,
            applied: false,
        })
    }
    pub fn apply(
        &self,
        writer: Writer,
        files: State<Option<ProjectFiles>>,
    ) -> Result<RefreshApplied, String> {
        let template = extraction::template(writer, files)?;
        let mut state = writer.localisation;
        let mut state = state.write();
        let catalogue = state.catalogue.as_mut().ok_or("Open a catalogue first.")?;
        self.source.apply(catalogue, &template)
    }
}
