use std::collections::BTreeSet;

use recite_core::{SourceId, SourceIdKind, is_valid_source_label};

use super::super::{AuthoringQuery, StableIdKind, StableIdSummary};

#[derive(Clone, Copy)]
pub(super) enum InsertionKind {
    FullId,
    AnchorOnly,
    AtAnchor,
}

pub(super) fn insertion_kind(stable: &StableIdSummary) -> Result<Option<InsertionKind>, ()> {
    match stable.source_id() {
        SourceId::Missing => Ok(Some(InsertionKind::FullId)),
        SourceId::Draft { .. } => Ok(Some(InsertionKind::AnchorOnly)),
        SourceId::Malformed { raw } if is_valid_source_label(raw) => {
            Ok(Some(InsertionKind::AtAnchor))
        }
        SourceId::Frozen { .. } => Ok(None),
        SourceId::Malformed { .. } => Err(()),
    }
}

pub(super) fn insertion_label(stable: &StableIdSummary, ordinal: u32) -> String {
    match stable.source_id() {
        SourceId::Draft { label } => label.clone(),
        SourceId::Malformed { raw } if is_valid_source_label(raw) => raw.clone(),
        _ => {
            let kind = match stable.kind() {
                StableIdKind::Line => "line",
                StableIdKind::Choice => "choice",
            };
            if ordinal == 1 {
                kind.to_owned()
            } else {
                format!("{kind}_{ordinal}")
            }
        }
    }
}

pub(super) fn occupied_anchors(
    snapshot: &AuthoringQuery<'_>,
) -> Result<BTreeSet<String>, crate::authoring::Interrupted> {
    let mut occupied = Vec::new();
    for document in snapshot.documents() {
        snapshot.checkpoint()?;
        for stable in document.summary().stable_ids() {
            if let Some(anchor) = stable.source_id().anchor() {
                occupied.push(anchor.as_str().to_owned());
            }
        }
    }
    // Bulk construction avoids repeated tree insertion while retaining per-document checkpoints.
    Ok(occupied.into_iter().collect())
}

pub(super) fn generated_anchor(
    occupied: &mut BTreeSet<String>,
    path: &str,
    kind: SourceIdKind,
    line: u32,
    column: u32,
    label: &str,
) -> Option<String> {
    for salt in 0..=u32::MAX {
        let candidate = SourceId::generated_anchor(path, kind, line, column, label, salt)
            .as_str()
            .to_owned();
        if occupied.insert(candidate.clone()) {
            return Some(candidate);
        }
    }
    None
}

pub(super) fn source_id_kind(kind: StableIdKind) -> SourceIdKind {
    match kind {
        StableIdKind::Line => SourceIdKind::Line,
        StableIdKind::Choice => SourceIdKind::Choice,
    }
}
