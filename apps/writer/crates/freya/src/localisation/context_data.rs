//! Extracted source order is context, not a claim about runtime traversal.
use recite_core::{PoCommentKind, PoDocument, PoEntry};
use std::collections::BTreeMap;
pub(super) fn metadata(entry: &PoEntry, key: &str) -> Option<String> {
    entry
        .comments()
        .iter()
        .filter(|c| *c.kind() == PoCommentKind::Extracted)
        .find_map(|c| c.text().strip_prefix(key).map(str::trim).map(str::to_owned))
}
pub(super) fn source(entry: &PoEntry) -> String {
    entry.plural_source_text().map_or_else(
        || entry.source_text().into(),
        |plural| format!("{}\n{plural}", entry.source_text()),
    )
}
pub(super) struct Nearby<'a> {
    entries: Vec<&'a PoEntry>,
    positions: BTreeMap<&'a str, usize>,
}
impl<'a> Nearby<'a> {
    pub(super) fn new(document: &'a PoDocument) -> Self {
        let entries: Vec<_> = document
            .entries()
            .iter()
            .filter(|e| !e.is_header() && !e.is_obsolete())
            .collect();
        let positions = entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.context().map(|c| (c, i)))
            .collect();
        Self { entries, positions }
    }
    pub(super) fn for_entry(&self, entry: &PoEntry) -> Vec<String> {
        let base = entry
            .context()
            .unwrap_or_default()
            .split('&')
            .next()
            .unwrap_or_default();
        let Some(&index) = self.positions.get(base) else {
            return Vec::new();
        };
        [index.checked_sub(1), index.checked_add(1)]
            .into_iter()
            .flatten()
            .filter_map(|i| self.entries.get(i))
            .filter(|e| {
                metadata(e, "file:") == metadata(entry, "file:")
                    && metadata(e, "block:") == metadata(entry, "block:")
                    && metadata(entry, "block:").is_some()
            })
            .map(|e| {
                metadata(e, "speaker:")
                    .map_or_else(|| source(e), |speaker| format!("{speaker}\n{}", source(e)))
            })
            .collect()
    }
}
