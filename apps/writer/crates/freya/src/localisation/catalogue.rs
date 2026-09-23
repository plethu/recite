//! File-backed PO editing. Drafts never become a parallel translation database.
mod recovery;
use super::messages::{MsgId, text as wording};
use recite_core::{PoDocument, PoDocumentFingerprint, PoEdit, PoEntryId};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct Draft {
    pub forms: Vec<String>,
    pub reviewed: bool,
}

pub(super) struct Comparison {
    pub rows: Vec<crate::design::ComparisonRow>,
    pub fingerprint: PoDocumentFingerprint,
}

pub(crate) struct Catalogue {
    pub path: PathBuf,
    pub document: PoDocument,
    baseline: PoDocumentFingerprint,
    drafts: BTreeMap<PoEntryId, Draft>,
    recovery: Option<crate::recovery::SnapshotStore<recovery::SnapshotData>>,
    recovery_failure: Option<String>,
    anchors: BTreeMap<String, Vec<PoEntryId>>,
}
impl Catalogue {
    pub fn open(path: &Path) -> Result<Self, String> {
        let text = crate::project::read_regular(path).map_err(|e| e.to_string())?;
        let document =
            PoDocument::parse_with_path(path.to_string_lossy(), text).map_err(|e| e.to_string())?;
        Ok(Self::from_document(path, document))
    }
    fn from_document(path: &Path, document: PoDocument) -> Self {
        let mut anchors: BTreeMap<String, Vec<PoEntryId>> = BTreeMap::new();
        for entry in document.entries() {
            if !entry.is_obsolete()
                && let Some(context) = entry.context()
            {
                anchors.entry(context.into()).or_default().push(entry.id());
            }
        }
        Self {
            recovery: None,
            recovery_failure: None,
            anchors,
            path: path.to_owned(),
            baseline: document.fingerprint(),
            document,
            drafts: BTreeMap::new(),
        }
    }
    pub fn dirty(&self) -> bool {
        !self.drafts.is_empty()
    }
    pub fn entry_for(&self, anchor: &str, source: &str) -> Option<PoEntryId> {
        let mut matches = self
            .anchors
            .get(anchor)?
            .iter()
            .filter_map(|id| self.document.entry(*id))
            .filter(|e| e.source_text() == source);
        let first = matches.next()?;
        matches.next().is_none().then_some(first.id())
    }
    pub(super) fn draft(&self, id: PoEntryId) -> Option<Draft> {
        self.drafts.get(&id).cloned().or_else(|| self.saved(id))
    }
    fn saved(&self, id: PoEntryId) -> Option<Draft> {
        let entry = self.document.entry(id)?;
        Some(Draft {
            forms: if entry.is_plural() {
                entry
                    .plural_translations()
                    .iter()
                    .map(|arm| arm.text().to_owned())
                    .collect()
            } else {
                vec![entry.translation()?.to_owned()]
            },
            reviewed: !entry.flags().iter().any(|f| f == "fuzzy")
                && if entry.is_plural() {
                    entry
                        .plural_translations()
                        .iter()
                        .all(|arm| !arm.text().trim().is_empty())
                } else {
                    entry.translation().is_some_and(|t| !t.trim().is_empty())
                },
        })
    }
    pub fn changed(&self, id: PoEntryId) -> bool {
        self.drafts.contains_key(&id)
    }
    pub(super) fn update(&mut self, id: PoEntryId, draft: Draft) {
        if self.saved(id).as_ref() == Some(&draft) {
            self.drafts.remove(&id);
        } else {
            self.drafts.insert(id, draft);
        }
        self.queue_recovery();
    }
    pub fn save(&mut self, id: PoEntryId) -> Result<(), String> {
        self.save_entries(&[id])
    }
    pub(super) fn save_all(&mut self) -> Result<(), String> {
        let ids: Vec<_> = self.drafts.keys().copied().collect();
        self.save_entries(&ids)
    }
    fn save_entries(&mut self, ids: &[PoEntryId]) -> Result<(), String> {
        if !ids.iter().any(|id| self.changed(*id)) {
            return Ok(());
        }
        let mut candidate = self.document.clone();
        for &id in ids {
            if let Some(draft) = self.drafts.get(&id) {
                self.apply_draft(&mut candidate, id, draft)?;
            }
        }
        crate::project::read_regular(&self.path).map_err(|e| e.to_string())?;
        let fingerprint = candidate
            .write_atomically(&self.path, &self.baseline)
            .map_err(|e| e.to_string())?;
        self.document = candidate;
        self.baseline = fingerprint;
        for id in ids {
            self.drafts.remove(id);
        }
        self.flush_recovery()
    }
    fn apply_draft(
        &self,
        candidate: &mut PoDocument,
        id: PoEntryId,
        draft: &Draft,
    ) -> Result<(), String> {
        candidate.set_fuzzy(id, true).map_err(|e| e.to_string())?;
        let entry = candidate
            .entry(id)
            .ok_or_else(|| wording(MsgId::WriterNoEntry))?;
        if entry.is_plural() {
            let count = self
                .plural_rule()
                .and_then(|rule| recite_core::validate_plural_rule(rule).ok())
                .ok_or_else(|| wording(MsgId::WriterEntryPluralInvalid))?;
            if draft.forms.len() != count {
                return Err(wording(MsgId::WriterEntryPluralInvalid));
            }
            for (index, value) in draft.forms.iter().enumerate() {
                candidate
                    .apply_edit(PoEdit::plural_translation(id, index, value))
                    .map_err(|e| e.to_string())?;
            }
        } else {
            let [value] = draft.forms.as_slice() else {
                return Err(wording(MsgId::WriterNoEntry));
            };
            candidate
                .apply_edit(PoEdit::translation(id, value))
                .map_err(|e| e.to_string())?;
        }
        if draft.reviewed {
            if draft.forms.iter().any(|text| text.trim().is_empty()) {
                return Err(wording(MsgId::WriterEmptyReview));
            }
            candidate.set_fuzzy(id, false).map_err(|e| e.to_string())?;
        }
        Ok(())
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

    pub fn reload(&mut self) -> Result<(), String> {
        if self.dirty() {
            return Err(wording(MsgId::WriterReloadDrafts));
        }
        self.adopt(Self::open(&self.path)?)
    }
    pub(super) fn compare(&self) -> Result<Comparison, String> {
        let disk = Self::open(&self.path)?;
        let mut rows = Vec::new();
        for (id, draft) in self.drafts.iter() {
            let Some(entry) = self.document.entry(*id) else {
                continue;
            };
            let target = entry
                .context()
                .and_then(|c| disk.entry_for(c, entry.source_text()))
                .and_then(|id| disk.draft(id));
            for (index, text) in draft.forms.iter().enumerate() {
                rows.push(crate::design::ComparisonRow {
                    caption: format!("{} · {}", entry.source_text(), index + 1),
                    before: Some(text.clone()),
                    after: target.as_ref().and_then(|d| d.forms.get(index)).cloned(),
                });
            }
        }
        Ok(Comparison {
            rows,
            fingerprint: disk.baseline,
        })
    }
    pub fn accept_external(
        &mut self,
        keep_drafts: bool,
        expected: &PoDocumentFingerprint,
    ) -> Result<(), String> {
        let mut next = Self::open(&self.path)?;
        if &next.baseline != expected {
            return Err(wording(MsgId::WriterCompare));
        }
        for (id, draft) in &self.drafts {
            let entry = self
                .document
                .entry(*id)
                .ok_or_else(|| wording(MsgId::WriterNoEntry))?;
            let target = entry
                .context()
                .and_then(|c| next.entry_for(c, entry.source_text()))
                .ok_or_else(|| wording(MsgId::WriterNoEntry))?;
            let target_entry = next
                .document
                .entry(target)
                .ok_or_else(|| wording(MsgId::WriterNoEntry))?;
            if target_entry.plural_source_text() != entry.plural_source_text()
                || (keep_drafts && self.plural_rule() != next.plural_rule())
            {
                return Err(wording(MsgId::WriterExternalStructureChanged));
            }
            let mut chosen = if keep_drafts {
                draft.clone()
            } else {
                next.draft(target)
                    .ok_or_else(|| wording(MsgId::WriterNoEntry))?
            };
            chosen.reviewed = false;
            next.drafts.insert(target, chosen);
        }
        self.adopt(next)
    }
    pub(super) fn replace_refreshed(
        &mut self,
        document: PoDocument,
        expected: &PoDocumentFingerprint,
    ) -> Result<(), String> {
        if self.dirty() {
            return Err(wording(MsgId::WriterReloadDrafts));
        }
        if &self.document.fingerprint() != expected {
            return Err(wording(MsgId::WriterCreationChanged));
        }
        crate::project::read_regular(&self.path).map_err(|e| e.to_string())?;
        document
            .write_atomically(&self.path, &self.baseline)
            .map_err(|e| e.to_string())?;
        self.adopt(Self::from_document(&self.path, document))
    }
    pub(super) fn plural_rule(&self) -> Option<&str> {
        self.document
            .headers()
            .iter()
            .find(|h| h.key().eq_ignore_ascii_case("Plural-Forms"))
            .map(|h| h.value())
    }
    pub fn discard(&mut self, id: PoEntryId) {
        self.drafts.remove(&id);
        self.queue_recovery();
    }
}

#[cfg(test)]
mod tests;
