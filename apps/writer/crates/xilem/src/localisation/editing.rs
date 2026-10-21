use super::*;
impl Writer {
    pub(super) fn update_translation(&mut self, key: &EntryKey, arm: usize, text: String) {
        if self.phase != Phase::Writing {
            return;
        }
        let Some(owner) = self
            .localisation
            .slot
            .owner_mut()
            .filter(|owner| owner.identity == key.catalogue)
        else {
            return;
        };
        let Some(mut draft) = owner.catalogue.draft(key.id) else {
            return;
        };
        let Some(form) = draft.forms.get_mut(arm) else {
            return;
        };
        *form = text;
        draft.reviewed = false;
        owner.catalogue.update(key.id, draft);
        self.status = "Translation draft updated; Save publishes the PO file.".into();
    }
    pub(super) fn review_translation(&mut self, key: &EntryKey, reviewed: bool) {
        if self.phase != Phase::Writing {
            return;
        }
        if let Some(owner) = self
            .localisation
            .slot
            .owner_mut()
            .filter(|owner| owner.identity == key.catalogue)
            && let Some(mut draft) = owner.catalogue.draft(key.id)
        {
            draft.reviewed = reviewed;
            owner.catalogue.update(key.id, draft);
        }
    }
    pub(super) fn discard_translation(&mut self, key: &EntryKey) {
        if self.phase != Phase::Writing {
            return;
        }
        if let Some(owner) = self
            .localisation
            .slot
            .owner_mut()
            .filter(|owner| owner.identity == key.catalogue)
        {
            owner.catalogue.discard(key.id);
        }
    }
}
