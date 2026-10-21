//! A source refresh binds the exact local catalogue and extracted source input.
use super::*;
use crate::localisation::refresh::changes;

pub struct RefreshPreview {
    document: PoDocument,
    origin: Arc<PoDocument>,
    revision: Arc<()>,
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
            && Arc::ptr_eq(&self.revision, &catalogue.revision)
            && !catalogue.dirty()
            && !catalogue.saving
    }
    pub fn apply(
        &self,
        catalogue: &mut Catalogue,
        current_template: &str,
    ) -> Result<RefreshApplied, String> {
        if current_template != self.template || !self.is_current(catalogue) {
            return Err(wording(MsgId::WriterCreationChanged));
        }
        match catalogue.replace_refreshed(self.document.clone(), &self.origin.fingerprint()) {
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
