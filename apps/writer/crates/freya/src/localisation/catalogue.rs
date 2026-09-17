//! File-backed PO editing. Drafts never become a parallel translation database.
use super::messages::{MsgId, text as wording};
use recite_core::{PoDocument, PoDocumentFingerprint, PoEdit, PoEntryId};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Draft {
    pub text: String,
    pub reviewed: bool,
}

pub(super) struct Comparison {
    pub text: String,
    pub fingerprint: PoDocumentFingerprint,
}

pub(crate) struct Catalogue {
    pub path: PathBuf,
    pub document: PoDocument,
    baseline: PoDocumentFingerprint,
    drafts: BTreeMap<PoEntryId, Draft>,
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
            .filter(|e| !e.is_plural() && e.source_text() == source);
        let first = matches.next()?;
        matches.next().is_none().then_some(first.id())
    }
    pub(super) fn draft(&self, id: PoEntryId) -> Option<Draft> {
        self.drafts.get(&id).cloned().or_else(|| self.saved(id))
    }
    fn saved(&self, id: PoEntryId) -> Option<Draft> {
        let entry = self.document.entry(id)?;
        Some(Draft {
            text: entry.translation()?.to_owned(),
            reviewed: !entry.flags().iter().any(|f| f == "fuzzy")
                && entry.translation().is_some_and(|t| !t.trim().is_empty()),
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
    }
    pub fn save(&mut self, id: PoEntryId) -> Result<(), String> {
        let Some(draft) = self.drafts.get(&id) else {
            return Ok(());
        };
        let mut candidate = self.document.clone();
        candidate.set_fuzzy(id, true).map_err(|e| e.to_string())?;
        candidate
            .apply_edit(PoEdit::translation(id, &draft.text))
            .map_err(|e| e.to_string())?;
        if draft.reviewed {
            if draft.text.trim().is_empty() {
                return Err(wording(MsgId::WriterEmptyReview));
            }
            candidate.set_fuzzy(id, false).map_err(|e| e.to_string())?;
        }
        crate::project::read_regular(&self.path).map_err(|e| e.to_string())?;
        let fingerprint = candidate
            .write_atomically(&self.path, &self.baseline)
            .map_err(|e| e.to_string())?;
        self.document = candidate;
        self.baseline = fingerprint;
        self.drafts.remove(&id);
        Ok(())
    }
    pub fn reload(&mut self) -> Result<(), String> {
        if self.dirty() {
            return Err(wording(MsgId::WriterReloadDrafts));
        }
        *self = Self::open(&self.path)?;
        Ok(())
    }
    pub(super) fn compare(&self) -> Result<Comparison, String> {
        let disk = Self::open(&self.path)?;
        let mut comparison = String::new();
        for (id, draft) in self.drafts.iter() {
            let Some(entry) = self.document.entry(*id) else {
                continue;
            };
            let target = entry
                .context()
                .and_then(|c| disk.entry_for(c, entry.source_text()))
                .and_then(|id| disk.draft(id));
            comparison.push_str(&format!(
                "{}\n\n{}\n\n{}\n\n",
                entry.source_text(),
                draft.text,
                target.map_or_else(|| "—".into(), |d| d.text)
            ));
        }
        Ok(Comparison {
            text: comparison,
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
        if keep_drafts {
            for (id, draft) in &self.drafts {
                let entry = self
                    .document
                    .entry(*id)
                    .ok_or_else(|| wording(MsgId::WriterNoEntry))?;
                let target = entry
                    .context()
                    .and_then(|c| next.entry_for(c, entry.source_text()))
                    .ok_or_else(|| wording(MsgId::WriterNoEntry))?;
                next.update(target, draft.clone());
            }
        }
        *self = next;
        Ok(())
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
        *self = Self::from_document(&self.path, document);
        Ok(())
    }
    pub fn discard(&mut self, id: PoEntryId) {
        self.drafts.remove(&id);
    }
}

#[cfg(test)]
mod tests;
