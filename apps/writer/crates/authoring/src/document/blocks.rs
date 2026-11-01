//! ID-only authoring operations must resolve one local declaration.
use super::{Document, EditError};

impl Document {
    /// Reading and editing an ID-only beat needs one projectable declaration.
    /// Recovery selection remains independent so preserved source can still open.
    pub fn validate_script_block(&self, id: &str) -> Result<(), EditError> {
        self.script_snapshot()?;
        self.unique_block_index(id).map(|_| ())
    }

    pub(crate) fn unique_block_index(&self, id: &str) -> Result<usize, EditError> {
        let mut matches = self
            .block_summaries()
            .iter()
            .enumerate()
            .filter(|(_, block)| block.id().as_str() == id);
        let (index, _) = matches.next().ok_or(EditError::Destination)?;
        if matches.next().is_some() {
            return Err(EditError::SourceRequired(
                "this beat name has more than one declaration",
            ));
        }
        Ok(index)
    }
}
