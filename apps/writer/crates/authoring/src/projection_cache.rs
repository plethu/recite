//! Immutable projections live exactly as long as the accepted source revision.
use crate::{Document, EditError, Passage, ScriptBlock};
use std::{cell::OnceCell, collections::BTreeMap, sync::Arc};

#[derive(Default)]
pub(crate) struct ProjectionCache {
    passages: OnceCell<Arc<[Passage]>>,
    script: OnceCell<Arc<[ScriptBlock]>>,
    ids: OnceCell<BTreeMap<String, usize>>,
}
impl Document {
    pub fn passage_snapshot(&self) -> Result<Arc<[Passage]>, EditError> {
        if let Some(value) = self.projections.passages.get() {
            return Ok(value.clone());
        }
        let value: Arc<[Passage]> = crate::projection::passages(self.source())?.into();
        Ok(self.projections.passages.get_or_init(|| value).clone())
    }
    pub fn script_snapshot(&self) -> Result<Arc<[ScriptBlock]>, EditError> {
        if let Some(value) = self.projections.script.get() {
            return Ok(value.clone());
        }
        let value: Arc<[ScriptBlock]> = self.project_script()?.into();
        Ok(self.projections.script.get_or_init(|| value).clone())
    }
    pub fn find_passage(&self, id: &str) -> Result<Option<Passage>, EditError> {
        let passages = self.passage_snapshot()?;
        let ids = self.projections.ids.get_or_init(|| {
            passages
                .iter()
                .enumerate()
                .map(|(index, passage)| (passage.id.clone(), index))
                .collect()
        });
        Ok(ids.get(id).map(|index| passages[*index].clone()))
    }
}
