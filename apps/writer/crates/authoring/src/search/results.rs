use super::{DocumentIndex, SearchHit};
use std::sync::Arc;

pub(super) struct SearchMatch {
    pub document: Arc<DocumentIndex>,
    pub id: usize,
}

/// Saved search results share prose and keep their originating shards alive.
/// Clones identify the same immutable snapshot; equality compares snapshot identity.
#[derive(Clone, Default)]
pub struct SearchResults {
    total: usize,
    hits: Arc<[SearchMatch]>,
}

impl PartialEq for SearchResults {
    fn eq(&self, other: &Self) -> bool {
        self.total == other.total && Arc::ptr_eq(&self.hits, &other.hits)
    }
}

impl SearchResults {
    pub(super) fn new(total: usize, hits: Vec<SearchMatch>) -> Self {
        Self {
            total,
            hits: hits.into(),
        }
    }

    pub const fn total(&self) -> usize {
        self.total
    }

    pub fn len(&self) -> usize {
        self.hits.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hits.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&SearchHit> {
        self.hits.get(index).map(|hit| &hit.document.hits[hit.id])
    }
}
