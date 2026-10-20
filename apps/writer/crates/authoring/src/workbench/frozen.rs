//! Read-only presentation during an exclusive project operation owns no edit history or preview.
use super::*;
pub struct FrozenWorkbench(Workbench);
impl Workbench {
    pub fn freeze(&self) -> FrozenWorkbench {
        let mut document = self.document.completion_snapshot().projection_document();
        document.projections = self.document.projections.capture_local().restore();
        FrozenWorkbench(Workbench {
            document,
            view: self.view.clone(),
            draft: self.draft.clone(),
            loaded: self.loaded.clone(),
            dirty: self.dirty,
            draft_revision: self.draft_revision,
            preview: None,
            page: None,
            script_selection: self.script_selection.clone(),
            source_validation: self.source_validation,
        })
    }
}
impl FrozenWorkbench {
    pub fn model(&self) -> &Workbench {
        &self.0
    }
}
#[cfg(test)]
mod tests;
