//! Local projections depend only on accepted source, not sibling analysis revisions.
use crate::{Document, EditError, Passage, ScriptBlock};
use recite_core::DocumentKey;
use std::{
    collections::BTreeMap,
    sync::{Arc, Weak},
};

#[derive(Clone)]
pub struct ProjectionRequest {
    key: DocumentKey,
    source: Arc<str>,
    passages: Option<Weak<[Passage]>>,
}

pub struct PreparedProjection {
    request: ProjectionRequest,
    passages: Arc<[Passage]>,
    script: Arc<[ScriptBlock]>,
    ids: Arc<BTreeMap<String, usize>>,
}

impl Document {
    /// A cache-only read never parses or waits for another thread.
    pub fn cached_passages(&self) -> Option<Arc<[Passage]>> {
        self.projections.passages.get().cloned()
    }
    pub fn cached_script(&self) -> Option<Arc<[ScriptBlock]>> {
        self.projections.script.get().cloned()
    }
    pub fn projection_request(&self) -> ProjectionRequest {
        ProjectionRequest {
            key: self.key().clone(),
            source: self.source_snapshot(),
            passages: self.cached_passages().as_ref().map(Arc::downgrade),
        }
    }
    /// Borrow the receipt so rejected or duplicate large results can be retired
    /// by their preparing thread. Accepted arrays remain shared with this cache.
    pub fn accept_projection(&self, prepared: &PreparedProjection) -> bool {
        if !prepared.request.is_current(self) {
            return false;
        }
        self.projections
            .passages
            .get_or_init(|| prepared.passages.clone());
        self.projections
            .script
            .get_or_init(|| prepared.script.clone());
        self.projections.ids.get_or_init(|| prepared.ids.clone());
        true
    }
}
impl ProjectionRequest {
    pub fn same_source(&self, other: &Self) -> bool {
        self.key == other.key && Arc::ptr_eq(&self.source, &other.source)
    }
    pub fn is_current(&self, document: &Document) -> bool {
        self.key == *document.key() && Arc::ptr_eq(&self.source, &document.source_snapshot())
    }
    pub fn prepare(&self) -> Result<PreparedProjection, EditError> {
        let parsed = recite_parser::parse(self.key.as_str(), self.source.as_ref());
        let lowered = parsed.lower_source_file();
        if !lowered.diagnostics.is_empty() {
            return Err(EditError::SourceRequired("repair the syntax diagnostics"));
        }
        // A queued request must not keep an old passage tree alive on the UI.
        let passages = match self.passages.as_ref().and_then(Weak::upgrade) {
            Some(passages) => passages,
            None => crate::projection::passages_from(&self.source, &parsed, &lowered)?.into(),
        };
        let script = crate::script::blocks(&self.source, &lowered, &passages).into();
        let ids = Arc::new(
            passages
                .iter()
                .enumerate()
                .map(|(index, passage)| (passage.id.clone(), index))
                .collect(),
        );
        Ok(PreparedProjection {
            request: self.clone(),
            passages,
            script,
            ids,
        })
    }
}
