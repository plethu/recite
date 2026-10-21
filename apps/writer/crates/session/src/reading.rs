//! Transient accepted reading snapshots, independent of an editor or native widget tree.
mod excerpt;
pub use excerpt::{Excerpt, ExcerptRow};
use recite_core::DocumentKey;
use recite_writer_model::{ScriptBlock, Workbench, WorkbenchError};

#[derive(Clone, PartialEq)]
pub struct ReferenceSnapshot {
    document: DocumentKey,
    block: ScriptBlock,
}
impl ReferenceSnapshot {
    /// Capture only accepted content; the caller resolves its outgoing field first.
    pub fn capture(model: &Workbench) -> Result<Option<Self>, WorkbenchError> {
        if model.has_draft() {
            return Err(WorkbenchError::DraftPending);
        }
        let Some(id) = model.selected_block()? else {
            return Ok(None);
        };
        let blocks = model.document().script_snapshot()?;
        Ok(blocks
            .iter()
            .find(|block| block.id == id)
            .map(|block| Self {
                document: model.document().key().clone(),
                block: block.clone(),
            }))
    }
    pub fn document(&self) -> &DocumentKey {
        &self.document
    }
    pub fn block(&self) -> &ScriptBlock {
        &self.block
    }
}
