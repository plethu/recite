//! Immutable catalogue data and a borrowed-key lookup order; no copied PO keys.
use recite_core::po::{PoDocument, PoEntry};

#[derive(Debug, Eq, PartialEq)]
pub(super) struct CatalogDocument {
    pub(super) document: PoDocument,
    order: Vec<usize>,
}
impl CatalogDocument {
    pub(super) fn new(document: PoDocument) -> Self {
        let mut order = Vec::with_capacity(document.entries().len());
        order.extend(
            (0..document.entries().len()).filter(|&index| !document.entries()[index].is_header()),
        );
        // Stable sorting preserves original order among duplicate draft keys.
        order.sort_by_key(|&index| key(&document.entries()[index]));
        Self { document, order }
    }

    pub(super) fn matching_entries<'a>(
        &'a self,
        context: &str,
        source: &str,
        plural: Option<&str>,
    ) -> impl Iterator<Item = &'a PoEntry> + use<'a> {
        let query = (Some(context), source, plural);
        let entries = self.document.entries();
        let start = self
            .order
            .partition_point(|&index| key(&entries[index]) < query);
        let end = self
            .order
            .partition_point(|&index| key(&entries[index]) <= query);
        self.order[start..end].iter().map(|&index| &entries[index])
    }
}
fn key(entry: &PoEntry) -> (Option<&str>, &str, Option<&str>) {
    (
        entry.context(),
        entry.source_text(),
        entry.plural_source_text(),
    )
}
