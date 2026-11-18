//! A source refresh binds the exact local catalogue and extracted source input.
use super::*;
use crate::localisation::refresh::changes;
use std::collections::BTreeSet;

pub struct RefreshPreview {
    document: PoDocument,
    origin: Arc<PoDocument>,
    revision: CatalogueRevision,
    path: PathBuf,
    template: String,
    pub summary: String,
    pub changes: Vec<changes::Change>,
}
pub struct RefreshApplied {
    pub recovery_error: Option<String>,
}
impl RefreshPreview {
    pub fn prepare(catalogue: &Catalogue, template: String) -> Result<Self, String> {
        catalogue.require_idle_save()?;
        if catalogue.dirty() {
            return Err(wording(MsgId::WriterReloadDrafts));
        }
        let extracted = PoDocument::parse(&template).map_err(|e| e.to_string())?;
        let document = catalogue
            .document
            .refreshed(&extracted)
            .map_err(|e| e.to_string())?;
        let changes = changes::collect(&catalogue.document, &document, &extracted);
        let count = |kind| changes.iter().filter(|change| change.kind == kind).count();
        let summary = format!(
            "{}: {} · {}: {} · {}: {}",
            wording(MsgId::WriterRefreshAdded),
            count(changes::Kind::Added),
            wording(MsgId::WriterRefreshChanged),
            count(changes::Kind::Changed),
            wording(MsgId::WriterRefreshRemoved),
            count(changes::Kind::Removed),
        );
        Ok(Self {
            document,
            origin: catalogue.document.clone(),
            revision: catalogue.revision.clone(),
            path: catalogue.path.clone(),
            template,
            summary,
            changes,
        })
    }
    pub fn is_current(&self, catalogue: &Catalogue) -> bool {
        self.path == catalogue.path
            && Arc::ptr_eq(&self.origin, &catalogue.document)
            && self.revision == catalogue.revision
            && !catalogue.dirty()
            && !catalogue.saving
    }
    pub fn apply(
        &self,
        catalogue: &mut Catalogue,
        current_template: &str,
    ) -> Result<RefreshApplied, String> {
        self.apply_keeping(catalogue, current_template, &BTreeSet::new())
    }
    /// Apply every update except the records whose context is in `kept`, which stay as they are.
    pub fn apply_keeping(
        &self,
        catalogue: &mut Catalogue,
        current_template: &str,
        kept: &BTreeSet<String>,
    ) -> Result<RefreshApplied, String> {
        if current_template != self.template || !self.is_current(catalogue) {
            return Err(wording(MsgId::WriterCreationChanged));
        }
        let document = if kept.is_empty() {
            self.document.clone()
        } else {
            let extracted = PoDocument::parse(&self.template).map_err(|e| e.to_string())?;
            self.origin
                .refreshed_keeping(&extracted, kept)
                .map_err(|e| e.to_string())?
        };
        match catalogue.replace_refreshed(document, &self.origin.fingerprint()) {
            Ok(()) => Ok(RefreshApplied {
                recovery_error: None,
            }),
            Err(error) if !Arc::ptr_eq(&self.origin, &catalogue.document) => Ok(RefreshApplied {
                recovery_error: Some(error),
            }),
            Err(error) => Err(error),
        }
    }
}

#[cfg(test)]
mod tests;
