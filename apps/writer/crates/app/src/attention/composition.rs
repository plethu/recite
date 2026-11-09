//! The native preedit lifetime synchronously prevents retirement of its writing surface.
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Clone, Default)]
pub struct Composition(Arc<AtomicUsize>);
impl PartialEq for Composition {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
pub struct Preedit(Arc<AtomicUsize>);
impl Composition {
    pub(crate) fn begin(&self) -> Preedit {
        self.0.fetch_add(1, Ordering::AcqRel);
        Preedit(self.0.clone())
    }
    pub(crate) fn active(&self) -> bool {
        self.0.load(Ordering::Acquire) != 0
    }
}
impl Drop for Preedit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
