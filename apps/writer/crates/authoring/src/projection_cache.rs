//! Immutable projections of accepted source and project analysis.
use crate::{Document, EditError, Passage, ScriptBlock};
use std::{
    cell::OnceCell,
    collections::BTreeMap,
    sync::{Arc, Weak},
};

#[cfg(test)]
mod tests;

mod preparing;
pub(crate) mod preview_bindings;
mod prose_region;
pub use preparing::{PreparedProjection, ProjectionRequest};

#[derive(Default)]
pub(crate) struct ProjectionCache {
    passages: OnceCell<Arc<[Passage]>>,
    script: OnceCell<Arc<[ScriptBlock]>>,
    // Unlike local passage/script projections, bindings also depend on siblings.
    bindings: OnceCell<Arc<[recite_core::ast::InterpolationBinding]>>,
    ids: OnceCell<Arc<BTreeMap<String, usize>>>,
}
pub(crate) struct CachedPassages {
    passages: Weak<[Passage]>,
    ids: Option<Weak<BTreeMap<String, usize>>>,
}
pub(crate) struct CachedProjections {
    passages: Option<CachedPassages>,
    script: Option<Weak<[ScriptBlock]>>,
}
impl CachedProjections {
    pub(crate) fn restore(&self) -> ProjectionCache {
        let cache = self
            .passages
            .as_ref()
            .map_or_else(ProjectionCache::default, CachedPassages::restore);
        if let Some(script) = self.script.as_ref().and_then(Weak::upgrade) {
            let _ = cache.script.set(script);
        }
        cache
    }
}
impl CachedPassages {
    pub(crate) fn restore(&self) -> ProjectionCache {
        let cache = ProjectionCache::default();
        // The ID map belongs to these exact rows. An expired passage hint must
        // take the normal cold path, even if an ID-map owner remains elsewhere.
        if let Some(passages) = self.passages.upgrade() {
            let _ = cache.passages.set(passages);
            if let Some(ids) = self.ids.as_ref().and_then(Weak::upgrade) {
                let _ = cache.ids.set(ids);
            }
        }
        cache
    }
}
impl ProjectionCache {
    pub(crate) fn capture_local(&self) -> CachedProjections {
        CachedProjections {
            passages: self.capture_passages(),
            script: self.script.get().map(Arc::downgrade),
        }
    }

    pub(crate) fn capture_passages(&self) -> Option<CachedPassages> {
        self.passages.get().map(|passages| CachedPassages {
            passages: Arc::downgrade(passages),
            ids: self.ids.get().map(Arc::downgrade),
        })
    }
    pub(crate) fn invalidate_project(&mut self) {
        self.bindings.take();
    }

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
            bindings: OnceCell::new(),
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
