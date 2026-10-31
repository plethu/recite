use recite_core::{SourcePosition, StableIdGenerator};

use super::helpers::{document, make_plan, point_range};
use super::stable_selection::{needs_insertion, occupied_ids, unoccupied_id};
use super::{AuthoringEditError, AuthoringEditOperation, AuthoringEditPlan, SourceEdit};
use crate::authoring::{AuthoringQuery, StableIdSummary};

/// Plans insertion of every missing stable ID in the effective project
/// snapshot, drawing new IDs from `ids`.
pub(super) fn plan_insert_missing_ids(
    snapshot: &AuthoringQuery<'_>,
    ids: &mut StableIdGenerator,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot.checkpoint()?;
    plan_insert(snapshot, ids, |_, _| true)
}

/// Plans insertion for the one stable-ID header at a source position.
pub(super) fn plan_insert_missing_id(
    snapshot: &AuthoringQuery<'_>,
    key: &recite_core::DocumentKey,
    position: SourcePosition,
    ids: &mut StableIdGenerator,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot.checkpoint()?;
    let document = document(snapshot, key)?;
    let matches = document
        .summary()
        .stable_ids()
        .iter()
        .filter(|stable| is_selected(stable, position))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Err(AuthoringEditError::NoSymbol {
            document: key.clone(),
            line: position.line(),
            column: position.column(),
        }),
        [_] => plan_insert(snapshot, ids, |candidate, stable| {
            candidate == key && is_selected(stable, position)
        }),
        _ => Err(AuthoringEditError::AmbiguousSymbol {
            document: key.clone(),
            line: position.line(),
            column: position.column(),
        }),
    }
}

impl AuthoringQuery<'_> {
    /// Plans insertion of all missing stable IDs.
    pub fn plan_insert_missing_ids(
        &self,
        ids: &mut StableIdGenerator,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_insert_missing_ids(self, ids)
    }

    /// Plans insertion of the missing stable ID at `position`.
    pub fn plan_insert_missing_id(
        &self,
        key: &recite_core::DocumentKey,
        position: SourcePosition,
        ids: &mut StableIdGenerator,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_insert_missing_id(self, key, position, ids)
    }
}

pub(super) fn plan_insert(
    snapshot: &AuthoringQuery<'_>,
    ids: &mut StableIdGenerator,
    select: impl Fn(&recite_core::DocumentKey, &StableIdSummary) -> bool,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot.checkpoint()?;
    let all_keys = snapshot
        .documents()
        .iter()
        .map(|document| document.key().clone())
        .collect::<Vec<_>>();
    for key in &all_keys {
        snapshot.checkpoint()?;
        let document = document(snapshot, key)?;
        if !document.participation().ast_structure().is_complete() {
            return Err(AuthoringEditError::Incomplete {
                document: key.clone(),
                class: crate::authoring::QueryClass::Diagnostics,
            });
        }
        let mut block_ids = document
            .summary()
            .blocks()
            .iter()
            .map(|block| block.id())
            .collect::<Vec<_>>();
        block_ids.sort();
        if let Some(pair) = block_ids.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(AuthoringEditError::AmbiguousBlock {
                block: pair[0].clone(),
            });
        }
    }

    let mut candidates = Vec::new();
    for document in snapshot.documents() {
        snapshot.checkpoint()?;
        for stable in document.summary().stable_ids() {
            snapshot.checkpoint()?;
            match needs_insertion(stable) {
                Ok(true) => {}
                Ok(false) => continue,
                Err(()) => {
                    return Err(AuthoringEditError::UnsupportedStableId {
                        document: document.key().clone(),
                    });
                }
            }
            if select(document.key(), stable) {
                candidates.push((document, stable));
            }
        }
    }

    if candidates.is_empty() {
        return Err(AuthoringEditError::NoEdits);
    }
    // Namespace construction is needed only when an ID will be generated.
    // Keep all completeness/ambiguity checks above even for a no-op request.
    let mut occupied = occupied_ids(snapshot)?;
    let mut edits = Vec::new();
    for (document, stable) in candidates {
        snapshot.checkpoint()?;
        let Some(span) = stable.insertion_span() else {
            return Err(AuthoringEditError::MissingSpan {
                document: document.key().clone(),
                role: "stable-ID insertion",
            });
        };
        let id = unoccupied_id(&mut occupied, ids);
        edits.push(SourceEdit::new(
            document.key().clone(),
            point_range(span.start),
            format!(" {}", id.field()),
        ));
    }
    make_plan(
        snapshot,
        all_keys,
        edits,
        AuthoringEditOperation::StableIdInsertion,
    )
}

fn is_selected(stable: &StableIdSummary, position: SourcePosition) -> bool {
    stable
        .source_id_span()
        .is_some_and(|span| super::helpers::position_in_span(span, position))
        || stable.span().start == position
        || stable
            .insertion_span()
            .is_some_and(|span| span.start == position)
}
