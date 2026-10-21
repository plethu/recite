//! External-file decisions bind both the exact local catalogue and the inspected disk version.
use super::*;
use crate::comparison::ComparisonRow;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExternalDecision {
    KeepDrafts,
    UseDisk,
}
#[derive(Clone)]
pub struct Inspection {
    path: PathBuf,
    baseline: PoDocumentFingerprint,
    document: Arc<PoDocument>,
    revision: CatalogueRevision,
    fingerprint: PoDocumentFingerprint,
}
pub struct Comparison {
    pub rows: Vec<ComparisonRow>,
    pub inspection: Inspection,
}
impl Catalogue {
    pub fn comparison_is_current(&self, inspection: &Inspection) -> bool {
        self.path == inspection.path
            && self.baseline == inspection.baseline
            && Arc::ptr_eq(&self.document, &inspection.document)
            && self.revision == inspection.revision
    }
    pub fn compare(&self) -> Result<Comparison, String> {
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
                rows.push(ComparisonRow {
                    caption: format!("{} · {}", entry.source_text(), index + 1),
                    before: Some(text.clone()),
                    after: target.as_ref().and_then(|d| d.forms.get(index)).cloned(),
                });
            }
        }
        Ok(Comparison {
            rows,
            inspection: Inspection {
                path: self.path.clone(),
                baseline: self.baseline.clone(),
                document: self.document.clone(),
                revision: self.revision.clone(),
                fingerprint: disk.baseline,
            },
        })
    }
    pub fn accept_external(
        &mut self,
        decision: ExternalDecision,
        inspection: &Inspection,
    ) -> Result<(), String> {
        self.require_idle_save()?;
        if !self.comparison_is_current(inspection) {
            return Err(
                "Translation drafts or the catalogue changed since comparison. Compare again."
                    .into(),
            );
        }
        let mut next = Self::open(&self.path)?;
        if next.baseline != inspection.fingerprint {
            return Err(wording(MsgId::WriterCatalogueChanged));
        }
        if decision == ExternalDecision::UseDisk {
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
}

#[cfg(test)]
mod tests;
