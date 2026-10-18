//! Project-wide preview inputs belong to the accepted analysis revision.
use crate::Document;
use recite_core::ast::InterpolationBinding;
use std::{collections::BTreeMap, sync::Arc};

impl Document {
    /// Caller-owned variables used by interpolation across the current project snapshot.
    /// Repeated calls share storage until accepted source or project inputs change;
    /// retained snapshots continue to describe their original revision.
    pub fn preview_bindings(&self) -> Arc<[InterpolationBinding]> {
        self.projections
            .bindings
            .get_or_init(|| collect(self.kernel()))
            .clone()
    }
}

pub(crate) fn collect(
    kernel: &recite_compiler::authoring::AuthoringKernel,
) -> Arc<[InterpolationBinding]> {
    let mut bindings = BTreeMap::new();
    for document in kernel.snapshot().documents() {
        for binding in document.summary().interpolation_bindings() {
            bindings.entry(binding.value.as_str()).or_insert(binding);
        }
    }
    bindings.into_values().cloned().collect::<Vec<_>>().into()
}
