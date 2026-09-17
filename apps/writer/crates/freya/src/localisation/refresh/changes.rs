//! Source-order change descriptions. Proximity is not runtime traversal.
use crate::localisation::{messages::MsgId, navigation::Destination};
use recite_core::{PoCommentKind, PoDocument, PoEntry};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Added,
    Changed,
    Removed,
}
impl Kind {
    pub fn label(self) -> MsgId {
        match self {
            Self::Added => MsgId::WriterChangeNew,
            Self::Changed => MsgId::WriterChangeChanged,
            Self::Removed => MsgId::WriterChangeRemoved,
        }
    }
}
pub(super) struct Change {
    pub kind: Kind,
    pub old: Option<String>,
    pub new: Option<String>,
    pub translation: String,
    pub caption: String,
    pub nearby: Vec<String>,
    pub notes: Vec<String>,
    pub destination: Option<Destination>,
}
pub(super) fn collect(
    before: &PoDocument,
    after: &PoDocument,
    template: &PoDocument,
) -> Vec<Change> {
    let old: BTreeMap<_, _> = before
        .entries()
        .iter()
        .filter(|e| !e.is_header() && !e.is_obsolete())
        .map(|e| (e.context(), e))
        .collect();
    let active: BTreeMap<_, _> = after
        .entries()
        .iter()
        .filter(|e| !e.is_header() && !e.is_obsolete())
        .map(|e| (e.context(), e))
        .collect();
    let context = Nearby::new(template);
    let previous_context = Nearby::new(before);
    let mut result = Vec::new();
    for next in after
        .entries()
        .iter()
        .filter(|e| !e.is_header() && !e.is_obsolete())
    {
        let previous = old.get(&next.context()).copied();
        let kind = match previous {
            None => Kind::Added,
            Some(e)
                if e.source_text() != next.source_text()
                    || e.plural_source_text() != next.plural_source_text() =>
            {
                Kind::Changed
            }
            _ => continue,
        };
        result.push(describe(kind, next, previous, Some(next), &context));
    }
    for entry in before
        .entries()
        .iter()
        .filter(|e| !e.is_header() && !e.is_obsolete())
    {
        if !active.contains_key(&entry.context()) {
            result.push(describe(
                Kind::Removed,
                entry,
                Some(entry),
                None,
                &previous_context,
            ));
        }
    }
    result
}
fn metadata(entry: &PoEntry, key: &str) -> Option<String> {
    entry
        .comments()
        .iter()
        .filter(|c| *c.kind() == PoCommentKind::Extracted)
        .find_map(|c| c.text().strip_prefix(key).map(str::trim).map(str::to_owned))
}
fn source(entry: &PoEntry) -> String {
    entry.plural_source_text().map_or_else(
        || entry.source_text().into(),
        |plural| format!("{}\n{plural}", entry.source_text()),
    )
}
fn describe(
    kind: Kind,
    entry: &PoEntry,
    old: Option<&PoEntry>,
    new: Option<&PoEntry>,
    context: &Nearby<'_>,
) -> Change {
    let nearby = context.for_entry(entry);
    Change {
        kind,
        old: old.map(source),
        new: new.map(source),
        translation: old.map_or_else(String::new, |e| {
            e.translation().map_or_else(
                || {
                    e.plural_translations()
                        .iter()
                        .map(|t| t.text())
                        .collect::<Vec<_>>()
                        .join("\n")
                },
                str::to_owned,
            )
        }),
        caption: [
            metadata(entry, "file:"),
            metadata(entry, "block:"),
            metadata(entry, "speaker:"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · "),
        nearby,
        notes: old
            .into_iter()
            .flat_map(|e| e.comments())
            .filter(|c| *c.kind() == PoCommentKind::Translator)
            .map(|c| c.text().to_owned())
            .collect(),
        destination: new.and_then(|e| Destination::resolve(e, &[])),
    }
}

struct Nearby<'a> {
    entries: Vec<&'a PoEntry>,
    positions: BTreeMap<&'a str, usize>,
}
impl<'a> Nearby<'a> {
    fn new(document: &'a PoDocument) -> Self {
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
    fn for_entry(&self, entry: &PoEntry) -> Vec<String> {
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
#[cfg(test)]
mod tests;
