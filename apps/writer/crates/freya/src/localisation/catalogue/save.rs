//! Atomic catalogue saves batch fields and final review flags before publication.
use super::*;
use recite_core::po::{PoEdit, PoUpdate};

impl Catalogue {
    pub fn save(&mut self, id: PoEntryId) -> Result<(), String> {
        self.save_entries(&[id])
    }
    pub(in crate::localisation) fn save_all(&mut self) -> Result<(), String> {
        let ids: Vec<_> = self.drafts.keys().copied().collect();
        self.save_entries(&ids)
    }
    fn save_entries(&mut self, ids: &[PoEntryId]) -> Result<(), String> {
        if !ids.iter().any(|id| self.changed(*id)) {
            return Ok(());
        }
        let mut candidate = self.document.clone();
        let mut updates = Vec::new();
        for &id in ids {
            if let Some(draft) = self.drafts.get(&id) {
                self.draft_updates(id, draft, &mut updates)?;
            }
        }
        candidate
            .apply_updates(updates)
            .map_err(|e| e.to_string())?;
        crate::project::read_regular(&self.path).map_err(|e| e.to_string())?;
        let fingerprint = candidate
            .write_atomically(&self.path, &self.baseline)
            .map_err(|e| e.to_string())?;
        self.revision = Arc::new(());
        self.document = candidate;
        self.baseline = fingerprint;
        for id in ids {
            self.drafts.remove(id);
        }
        self.flush_recovery()
    }
    fn draft_updates(
        &self,
        id: PoEntryId,
        draft: &Draft,
        updates: &mut Vec<PoUpdate>,
    ) -> Result<(), String> {
        let entry = self
            .document
            .entry(id)
            .ok_or_else(|| wording(MsgId::WriterNoEntry))?;
        if draft.reviewed && draft.forms.iter().any(|text| text.trim().is_empty()) {
            return Err(wording(MsgId::WriterEmptyReview));
        }
        if entry.is_plural() {
            let count = self
                .plural_rule()
                .and_then(|rule| recite_core::po::validate_plural_rule(rule).ok())
                .ok_or_else(|| wording(MsgId::WriterEntryPluralInvalid))?;
            if draft.forms.len() != count {
                return Err(wording(MsgId::WriterEntryPluralInvalid));
            }
            updates.extend(draft.forms.iter().enumerate().map(|(index, value)| {
                PoUpdate::Field(PoEdit::plural_translation(id, index, value))
            }));
        } else {
            let [value] = draft.forms.as_slice() else {
                return Err(wording(MsgId::WriterNoEntry));
            };
            updates.push(PoUpdate::Field(PoEdit::translation(id, value)));
        }
        updates.push(PoUpdate::Fuzzy {
            entry: id,
            fuzzy: !draft.reviewed,
        });
        Ok(())
    }
}
