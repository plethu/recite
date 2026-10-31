use std::collections::BTreeSet;

use recite_core::{SourceId, StableId, StableIdGenerator};

use super::super::{AuthoringQuery, StableIdSummary};

/// Whether tooling can assign an ID to this header.
///
/// Only a missing ID is assigned. Malformed IDs need an author decision, and
/// frozen IDs are never rewritten.
pub(super) fn needs_insertion(stable: &StableIdSummary) -> Result<bool, ()> {
    match stable.source_id() {
        SourceId::Missing => Ok(true),
        SourceId::Frozen(_) => Ok(false),
        SourceId::Malformed { .. } => Err(()),
    }
}

pub(super) fn occupied_ids(
    snapshot: &AuthoringQuery<'_>,
) -> Result<BTreeSet<StableId>, crate::authoring::Interrupted> {
    let mut occupied = Vec::new();
    for document in snapshot.documents() {
        snapshot.checkpoint()?;
        for stable in document.summary().stable_ids() {
            if let Some(id) = stable.source_id().stable_id() {
                occupied.push(id.clone());
            }
        }
    }
    // Bulk construction avoids repeated tree insertion while retaining per-document checkpoints.
    Ok(occupied.into_iter().collect())
}

/// Draws generated IDs until one is free in the project namespace.
pub(super) fn unoccupied_id(
    occupied: &mut BTreeSet<StableId>,
    ids: &mut StableIdGenerator,
) -> StableId {
    loop {
        let candidate = ids.next_id();
        if occupied.insert(candidate.clone()) {
            return candidate;
        }
    }
}
