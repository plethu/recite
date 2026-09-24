//! Preview a source refresh before replacing the shared catalogue.
use super::{
    extraction,
    messages::{MsgId, text as wording},
};
use crate::{editing::Writer, project::ProjectFiles};
use freya::prelude::State;
use recite_core::{PoDocument, PoDocumentFingerprint};
use std::path::PathBuf;
mod changes;
mod detail;

pub(super) mod screen;

pub(crate) struct Preview {
    document: PoDocument,
    baseline: PoDocumentFingerprint,
    path: PathBuf,
    template: String,
    pub summary: String,
    applied: bool,
    changes: Vec<changes::Change>,
}
impl Preview {
    pub fn prepare(writer: Writer, files: State<Option<ProjectFiles>>) -> Result<Self, String> {
        if writer.localisation.peek().dirty() {
            return Err(wording(MsgId::WriterReloadDrafts));
        }
        let template = extraction::template(writer, files)?;
        let state = writer.localisation.peek();
        let catalogue = state
            .catalogue
            .as_ref()
            .ok_or_else(|| wording(MsgId::WriterNoEntry))?;
        let extracted = PoDocument::parse(&template).map_err(|e| e.to_string())?;
        let document = catalogue
            .document
            .refreshed(&extracted)
            .map_err(|e| e.to_string())?;
        let changes = changes::collect(&catalogue.document, &document, &extracted);
        let added = changes
            .iter()
            .filter(|c| c.kind == changes::Kind::Added)
            .count();
        let changed = changes
            .iter()
            .filter(|c| c.kind == changes::Kind::Changed)
            .count();
        let removed = changes
            .iter()
            .filter(|c| c.kind == changes::Kind::Removed)
            .count();
        let summary = format!(
            "{}: {added} · {}: {changed} · {}: {removed}",
            wording(MsgId::WriterRefreshAdded),
            wording(MsgId::WriterRefreshChanged),
            wording(MsgId::WriterRefreshRemoved)
        );
        Ok(Self {
            document,
            baseline: catalogue.document.fingerprint(),
            path: catalogue.path.clone(),
            template,
            summary,
            changes,
            applied: false,
        })
    }
    pub fn apply(&self, writer: Writer, files: State<Option<ProjectFiles>>) -> Result<(), String> {
        if extraction::template(writer, files)? != self.template {
            return Err(wording(MsgId::WriterCreationChanged));
        }
        let mut state = writer.localisation;
        let mut state = state.write();
        let catalogue = state
            .catalogue
            .as_mut()
            .ok_or_else(|| wording(MsgId::WriterNoEntry))?;
        if catalogue.path != self.path {
            return Err(wording(MsgId::WriterCreationChanged));
        }
        catalogue.replace_refreshed(self.document.clone(), &self.baseline)
    }
}
