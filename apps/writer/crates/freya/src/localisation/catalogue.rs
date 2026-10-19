//! File-backed PO editing. Drafts never become a parallel translation database.
mod preview;
mod queue;
pub(crate) use preview::PreviewInput;
mod recovery;
mod save;
use super::messages::{MsgId, text as wording};
use recite_core::po::{PoDocument, PoDocumentFingerprint, PoEntryId};
pub(crate) use save::{CatalogueDurability, CatalogueSave, CatalogueSaved};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
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
    pub document: Arc<PoDocument>,
    pub(crate) saving: bool,
    baseline: PoDocumentFingerprint,
    // Retained by trial runs; replacement also invalidates snapshots after reload.
    revision: Arc<()>,
    drafts: BTreeMap<PoEntryId, Arc<Draft>>,
    recovery: Option<crate::recovery::SnapshotStore<recovery::SnapshotData>>,
    recovery_failure: Option<String>,
    anchors: BTreeMap<String, Vec<PoEntryId>>,
    queue_cache: std::cell::RefCell<Option<queue::QueueCache>>,
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
            revision: Arc::new(()),
            recovery: None,
            recovery_failure: None,
            anchors,
            queue_cache: Default::default(),
            path: path.to_owned(),
            baseline: document.fingerprint(),
            document: Arc::new(document),
            saving: false,
            drafts: BTreeMap::new(),
        }
    }
    pub(super) fn saved_source(&self) -> Arc<String> {
        self.document.source_snapshot()
    }
    pub(crate) fn saved_fingerprint(&self) -> &PoDocumentFingerprint {
        &self.baseline
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
        self.drafts
            .get(&id)
            .map(|draft| draft.as_ref().clone())
            .or_else(|| self.saved(id))
    }
    pub(super) fn forms(&self, id: PoEntryId) -> impl Iterator<Item = &str> {
        let draft = self.drafts.get(&id);
        draft
            .into_iter()
            .flat_map(|d| d.forms.iter().map(String::as_str))
            .chain(
                self.document
                    .entry(id)
                    .filter(|_| draft.is_none())
                    .into_iter()
                    .flat_map(|entry| {
                        entry
                            .translation()
                            .into_iter()
                            .chain(entry.plural_translations().iter().map(|arm| arm.text()))
                    }),
            )
    }
    pub(super) fn reviewed(&self, id: PoEntryId) -> bool {
        self.drafts.get(&id).map_or_else(
            || {
                self.document.entry(id).is_some_and(|entry| {
                    !entry.flags().iter().any(|flag| flag == "fuzzy")
                        && self.forms(id).all(|text| !text.trim().is_empty())
                })
            },
            |draft| draft.reviewed,
        )
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
            self.drafts.insert(id, Arc::new(draft));
        }
        self.revision = Arc::new(());
        self.queue_recovery();
    }
    pub fn reload(&mut self) -> Result<(), String> {
        self.require_idle_save()?;
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
        self.require_idle_save()?;
        let mut next = Self::open(&self.path)?;
        if &next.baseline != expected {
            return Err(wording(MsgId::WriterCompare));
        }
        if !keep_drafts {
            return self.adopt(next);
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
                || self.plural_rule() != next.plural_rule()
            {
                return Err(wording(MsgId::WriterExternalStructureChanged));
            }
            let mut chosen = draft.as_ref().clone();
            chosen.reviewed = false;
            next.drafts.insert(target, Arc::new(chosen));
        }
        self.adopt(next)
    }
    pub(super) fn replace_refreshed(
        &mut self,
        document: PoDocument,
        expected: &PoDocumentFingerprint,
    ) -> Result<(), String> {
        self.require_idle_save()?;
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
        if self.saving {
            return;
        }
        self.drafts.remove(&id);
        self.revision = Arc::new(());
        self.queue_recovery();
    }
}

#[cfg(test)]
mod tests;
