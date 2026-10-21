//! File-backed PO editing. Drafts never become a parallel translation database.
mod entry_details;
mod preview;
pub use entry_details::{EntryDetails, PluralGuide, Wording};
mod queue;
mod refresh;
pub use preview::PreviewInput;
pub use queue::CatalogueQueue;
pub use refresh::{RefreshApplied, RefreshPreview};
mod recovery;
mod save;
use super::messages::{MsgId, text as wording};
use recite_core::po::{PoDocument, PoDocumentFingerprint, PoEntryId};
use recite_writer_model::CatalogueRevision;
pub use save::{CatalogueDurability, CatalogueSave, CatalogueSaved};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Draft {
    pub forms: Vec<String>,
    pub reviewed: bool,
}

mod external;
pub use external::{Comparison, ExternalDecision, Inspection};

pub struct Catalogue {
    pub path: PathBuf,
    pub document: Arc<PoDocument>,
    pub saving: bool,
    baseline: PoDocumentFingerprint,
    // Retained by trial runs; replacement also invalidates snapshots after reload.
    revision: CatalogueRevision,
    drafts: BTreeMap<PoEntryId, Arc<Draft>>,
    recovery: Option<crate::recovery::SnapshotStore<recovery::SnapshotData>>,
    recovery_failure: Option<String>,
    anchors: BTreeMap<String, Vec<PoEntryId>>,
    queue_cache: std::cell::RefCell<Option<queue::QueueCache>>,
    entry_cache: std::cell::RefCell<Option<entry_details::Cache>>,
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
            revision: CatalogueRevision::default(),
            recovery: None,
            recovery_failure: None,
            anchors,
            queue_cache: Default::default(),
            entry_cache: Default::default(),
            path: path.to_owned(),
            baseline: document.fingerprint(),
            document: Arc::new(document),
            saving: false,
            drafts: BTreeMap::new(),
        }
    }
    pub fn saved_source(&self) -> Arc<String> {
        self.document.source_snapshot()
    }
    pub fn saved_fingerprint(&self) -> &PoDocumentFingerprint {
        &self.baseline
    }
    pub fn dirty(&self) -> bool {
        !self.drafts.is_empty()
    }
    pub fn changed_entries(&self) -> impl ExactSizeIterator<Item = PoEntryId> + '_ {
        self.drafts.keys().copied()
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
    pub fn draft(&self, id: PoEntryId) -> Option<Draft> {
        self.drafts
            .get(&id)
            .map(|draft| draft.as_ref().clone())
            .or_else(|| self.saved(id))
    }
    pub fn forms(&self, id: PoEntryId) -> impl Iterator<Item = &str> {
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
    pub fn reviewed(&self, id: PoEntryId) -> bool {
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
    pub fn update(&mut self, id: PoEntryId, draft: Draft) {
        if self.saved(id).as_ref() == Some(&draft) {
            self.drafts.remove(&id);
        } else {
            self.drafts.insert(id, Arc::new(draft));
        }
        self.revision = CatalogueRevision::default();
        self.queue_recovery();
    }
    pub fn reload(&mut self) -> Result<(), String> {
        self.require_idle_save()?;
        if self.dirty() {
            return Err(wording(MsgId::WriterReloadDrafts));
        }
        self.adopt(Self::open(&self.path)?)
    }
    pub fn replace_refreshed(
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
    pub fn plural_rule(&self) -> Option<&str> {
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
        self.revision = CatalogueRevision::default();
        self.queue_recovery();
    }
}

#[cfg(test)]
mod tests;
