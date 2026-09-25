//! Source-order change descriptions. Proximity is not runtime traversal.
use crate::localisation::context_data::{Nearby, metadata, source};
use crate::localisation::{messages::MsgId, navigation::Destination};
use recite_core::po::{PoCommentKind, PoDocument, PoEntry};
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

#[cfg(test)]
mod tests;
