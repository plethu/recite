//! Immutable catalogue inputs and revision identity for trial runs.
use super::*;
use recite_core::po::{PoEdit, PoUpdate};

impl Catalogue {
    pub(crate) fn revision(&self) -> std::sync::Arc<()> {
        self.revision.clone()
    }
    pub(crate) fn preview_document(&self, include_drafts: bool) -> Result<PoDocument, String> {
        let disk = Self::open(&self.path)?;
        if !include_drafts {
            return Ok(disk.document);
        }
        if disk.baseline != self.baseline {
            return Err(wording(MsgId::WriterCompare));
        }
        let mut document = self.document.clone();
        let mut updates = Vec::new();
        for (id, draft) in &self.drafts {
            let plural = document.entry(*id).is_some_and(|entry| entry.is_plural());
            for (index, value) in draft.forms.iter().enumerate() {
                let edit = if plural {
                    PoEdit::plural_translation(*id, index, value)
                } else {
                    PoEdit::translation(*id, value)
                };
                updates.push(PoUpdate::Field(edit));
            }
            updates.push(PoUpdate::Fuzzy {
                entry: *id,
                fuzzy: false,
            });
        }
        document.apply_updates(updates).map_err(|e| e.to_string())?;
        Ok(document)
    }
}

#[cfg(test)]
mod tests;
