use toml_edit::{DocumentMut, Item, Table, value};

/// Source-only normalization before the canonical manifest raw layer.
///
/// The source format allows the fixed producer kind to be omitted as a
/// convenience, while the format-neutral manifest raw model keeps it
/// explicit. The clone is never exposed or retained as editable state.
pub(super) fn canonical_document(document: &DocumentMut) -> DocumentMut {
    let mut canonical = document.clone();
    if let Some(Item::Table(producer)) = canonical.get_mut("producer") {
        ensure_producer_kind(producer);
    }
    canonical
}

fn ensure_producer_kind(producer: &mut Table) {
    if !producer.contains_key("kind") {
        producer.insert("kind", value("standalone"));
    }
}

pub(super) const GENERATED_ONLY_FIELDS: &[&str] = &[
    "content_fingerprint",
    "schema_export_version",
    "inclusion_policy",
    "producer_fingerprints",
];
