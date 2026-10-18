//! Trial publication stays with the revision-checked workbench.
use super::{Workbench, WorkbenchError};
use crate::{PreparedPreview, Preview, PreviewRequest, PreviewSetup};

impl Workbench {
    pub fn preview_request(&self) -> Result<PreviewRequest, WorkbenchError> {
        self.require_applied()?;
        Ok(PreviewRequest::capture(
            &self.document,
            self.selected_block()?,
        ))
    }
    /// Reject stale preparation without changing the running trial. The caller retains
    /// rejected/replaced sessions so their destruction can happen off the UI thread.
    pub fn accept_preview(
        &mut self,
        prepared: Box<PreparedPreview>,
    ) -> Result<Option<Preview>, Box<PreparedPreview>> {
        if !prepared.request.is_current(self) {
            return Err(prepared);
        }
        self.page = Some(prepared.page);
        Ok(self.preview.replace(prepared.preview))
    }
    pub fn start_preview(&mut self) -> Result<(), WorkbenchError> {
        self.start_preview_with(PreviewSetup::default())
    }
    pub fn start_preview_with(&mut self, setup: PreviewSetup) -> Result<(), WorkbenchError> {
        let prepared = self.preview_request()?.prepare(setup)?;
        // No state can change between capture and publication in this synchronous API.
        self.preview = Some(prepared.preview);
        self.page = Some(prepared.page);
        Ok(())
    }
}
