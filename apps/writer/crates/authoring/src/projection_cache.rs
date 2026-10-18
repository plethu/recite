//! Immutable projections live exactly as long as the accepted source revision.
use crate::{Document, EditError, Passage, ScriptBlock};
use std::{cell::OnceCell, collections::BTreeMap, sync::Arc};

mod prose_region;

#[derive(Default)]
pub(crate) struct ProjectionCache {
    passages: OnceCell<Arc<[Passage]>>,
    script: OnceCell<Arc<[ScriptBlock]>>,
    ids: OnceCell<Arc<BTreeMap<String, usize>>>,
}
impl ProjectionCache {
    pub(crate) fn for_prose_edit(
        &self,
        source: &str,
        lowered: &recite_parser::LoweredSourceFile,
        passages: Vec<Passage>,
    ) -> Self {
        // Preserve demand: passage-only clients do not eagerly construct Script.
        let script = self
            .script
            .get()
            .map(|_| Arc::<[ScriptBlock]>::from(crate::script::blocks(source, lowered, &passages)));
        Self {
            passages: OnceCell::from(Arc::<[Passage]>::from(passages)),
            script: script.map_or_else(OnceCell::new, OnceCell::from),
            ids: OnceCell::new(),
        }
    }
}
impl Document {
    pub fn passage_snapshot(&self) -> Result<Arc<[Passage]>, EditError> {
        if let Some(value) = self.projections.passages.get() {
            return Ok(value.clone());
        }
        let value: Arc<[Passage]> = crate::projection::passages(self.source())?.into();
        Ok(self.projections.passages.get_or_init(|| value).clone())
    }
    pub(crate) fn passage_snapshot_from(
        &self,
        parsed: &recite_parser::Parse,
        lowered: &recite_parser::LoweredSourceFile,
    ) -> Result<Arc<[Passage]>, EditError> {
        if let Some(value) = self.projections.passages.get() {
            return Ok(value.clone());
        }
        let value = crate::projection::passages_from(self.source(), parsed, lowered)?.into();
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
            Arc::new(
                passages
                    .iter()
                    .enumerate()
                    .map(|(index, passage)| (passage.id.clone(), index))
                    .collect(),
            )
        });
        Ok(ids.get(id).map(|index| passages[*index].clone()))
    }
}
