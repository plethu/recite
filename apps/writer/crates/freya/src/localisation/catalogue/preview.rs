//! Immutable catalogue inputs and revision identity for trial runs.
use super::*;

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
        for (id, draft) in &self.drafts {
            document.set_fuzzy(*id, true).map_err(|e| e.to_string())?;
            let plural = document.entry(*id).is_some_and(|entry| entry.is_plural());
            for (index, value) in draft.forms.iter().enumerate() {
                let edit = if plural {
                    PoEdit::plural_translation(*id, index, value)
                } else {
                    PoEdit::translation(*id, value)
                };
                document.apply_edit(edit).map_err(|e| e.to_string())?;
            }
            document.set_fuzzy(*id, false).map_err(|e| e.to_string())?;
        }
        Ok(document)
    }
}

#[cfg(test)]
mod tests;
