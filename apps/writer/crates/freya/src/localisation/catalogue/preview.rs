//! Immutable catalogue inputs and revision identity for trial runs.
use super::*;
use recite_core::po::{PoEdit, PoUpdate};

impl Catalogue {
    pub(crate) fn revision(&self) -> std::sync::Arc<()> {
        self.revision.clone()
    }
    pub(crate) fn preview_input(&self, include_drafts: bool) -> PreviewInput {
        PreviewInput {
            path: self.path.clone(),
            baseline: self.baseline.clone(),
            drafts: include_drafts.then(|| self.drafts.clone()),
        }
    }
}

pub(crate) struct PreviewInput {
    path: PathBuf,
    baseline: PoDocumentFingerprint,
    drafts: Option<BTreeMap<PoEntryId, Arc<Draft>>>,
}
impl PreviewInput {
    pub(crate) fn prepare(&self) -> Result<PoDocument, String> {
        let text = crate::project::read_regular(&self.path).map_err(|e| e.to_string())?;
        let mut document = PoDocument::parse_with_path(self.path.to_string_lossy(), text)
            .map_err(|e| e.to_string())?;
        let Some(drafts) = &self.drafts else {
            return Ok(document);
        };
        if document.fingerprint() != self.baseline {
            return Err(wording(MsgId::WriterCompare));
        }
        // Equal fingerprints mean captured entry IDs still address these bytes.
        // Reuse the parsed disk document; no second PO parse or navigation index.
        let mut updates = Vec::new();
        for (id, draft) in drafts {
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
