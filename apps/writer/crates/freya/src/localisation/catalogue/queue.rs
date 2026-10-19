//! Local translation saves retain structural queue rows and patch saved forms.
use super::Catalogue;
use crate::localisation::queue::filter::QueueIndex;
use recite_writer_model::Passage;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Weak},
};

pub(super) struct QueueCache {
    passages: Option<Weak<[Passage]>>,
    pub(super) index: Rc<RefCell<QueueIndex>>,
}
impl Catalogue {
    pub(in crate::localisation) fn queue_index(
        &self,
        passages: Option<Arc<[Passage]>>,
    ) -> Rc<RefCell<QueueIndex>> {
        let mut cache = self.queue_cache.borrow_mut();
        let same = cache
            .as_ref()
            .is_some_and(|cache| match (&cache.passages, &passages) {
                (Some(old), Some(new)) => Weak::ptr_eq(old, &Arc::downgrade(new)),
                (None, None) => true,
                _ => false,
            });
        if !same {
            let index = QueueIndex::new(self, passages.as_deref().unwrap_or_default());
            *cache = Some(QueueCache {
                passages: passages.as_ref().map(Arc::downgrade),
                index: Rc::new(RefCell::new(index)),
            });
        }
        cache
            .as_ref()
            .unwrap_or_else(|| unreachable!("cache initialized above"))
            .index
            .clone()
    }
}
