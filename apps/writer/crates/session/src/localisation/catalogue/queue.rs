//! Local translation saves retain structural queue rows and patch saved forms.
use super::Catalogue;
use crate::localisation::queue::filter::QueueIndex;
use recite_core::po::PoEntryId;
use recite_writer_model::Passage;
use std::sync::{Arc, Mutex, Weak};

/// A shared derived queue; the catalogue remains its sole source of truth.
#[derive(Clone, Default)]
pub struct CatalogueQueue(Arc<Mutex<QueueIndex>>);
impl PartialEq for CatalogueQueue {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl CatalogueQueue {
    pub fn matching_entries(
        &self,
        catalogue: &Catalogue,
        query: &str,
        only_attention: bool,
    ) -> Result<Vec<PoEntryId>, String> {
        self.0
            .lock()
            .map_err(|_| {
                "The catalogue queue cache stopped unexpectedly. Reopen the queue to rebuild it."
                    .to_owned()
            })
            .map(|index| index.matching_entries(catalogue, query, only_attention))
    }
    pub(super) fn replace_forms(&self, updates: &mut [(PoEntryId, Vec<String>)]) -> Result<(), ()> {
        self.0
            .lock()
            .map_err(|_| ())
            .map(|mut index| index.replace_forms(updates))
    }
}

pub struct QueueCache {
    passages: Option<Weak<[Passage]>>,
    pub index: CatalogueQueue,
}
impl Catalogue {
    pub fn queue_index(&self, passages: Option<Arc<[Passage]>>) -> CatalogueQueue {
        let mut cache = self.queue_cache.borrow_mut();
        let same = cache.as_ref().is_some_and(|cache| {
            !cache.index.0.is_poisoned()
                && match (&cache.passages, &passages) {
                    (Some(old), Some(new)) => Weak::ptr_eq(old, &Arc::downgrade(new)),
                    (None, None) => true,
                    _ => false,
                }
        });
        if !same {
            let index = QueueIndex::new(self, passages.as_deref().unwrap_or_default());
            *cache = Some(QueueCache {
                passages: passages.as_ref().map(Arc::downgrade),
                index: CatalogueQueue(Arc::new(Mutex::new(index))),
            });
        }
        cache
            .as_ref()
            .unwrap_or_else(|| unreachable!("cache initialized above"))
            .index
            .clone()
    }
}
#[cfg(test)]
mod tests;
