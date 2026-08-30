use std::collections::BTreeMap;

use recite_core::{DocumentKey, SourcePosition};

use super::helpers::{document, make_plan, point_range};
use super::stable_selection::{
    InsertionKind, generated_anchor, insertion_kind, insertion_label, occupied_anchors,
    source_id_kind,
};
use super::{AuthoringEditError, AuthoringEditOperation, AuthoringEditPlan, SourceEdit};
use crate::authoring::{AuthoringSnapshot, StableIdKind, StableIdSummary};

/// Plans deterministic insertion of every missing or draft stable ID in the
/// effective project snapshot.
pub fn plan_insert_missing_ids(
    snapshot: &AuthoringSnapshot,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    let keys = snapshot
        .documents()
        .iter()
        .map(|document| document.key().clone())
        .collect::<Vec<_>>();
    plan_insert(snapshot, keys, |_| true)
}

/// Plans insertion for the one stable-ID header at a source position.
pub fn plan_insert_missing_id(
    snapshot: &AuthoringSnapshot,
    key: &recite_core::DocumentKey,
    position: SourcePosition,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
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
        [_] => plan_insert(snapshot, vec![key.clone()], |stable| {
            is_selected(stable, position)
        }),
        _ => Err(AuthoringEditError::AmbiguousSymbol {
            document: key.clone(),
            line: position.line(),
            column: position.column(),
        }),
    }
}

impl AuthoringSnapshot {
    /// Plans deterministic insertion of all missing stable IDs.
    pub fn plan_insert_missing_ids(&self) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_insert_missing_ids(self)
    }

    /// Plans insertion of the missing stable ID at `position`.
    pub fn plan_insert_missing_id(
        &self,
        key: &recite_core::DocumentKey,
        position: SourcePosition,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_insert_missing_id(self, key, position)
    }
}

fn plan_insert(
    snapshot: &AuthoringSnapshot,
    keys: Vec<recite_core::DocumentKey>,
    select: impl Fn(&StableIdSummary) -> bool,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    let all_keys = snapshot
        .documents()
        .iter()
        .map(|document| document.key().clone())
        .collect::<Vec<_>>();
    for key in &all_keys {
        if !document(snapshot, key)?
            .participation()
            .ast_structure()
            .is_complete()
        {
            return Err(AuthoringEditError::Incomplete {
                document: key.clone(),
                class: crate::authoring::QueryClass::Diagnostics,
            });
        }
    }

    let mut occupied = occupied_anchors(snapshot);
    let mut ordinals = BTreeMap::<(DocumentKey, String, StableIdKind), u32>::new();
    let mut candidates = Vec::new();
    for document in snapshot.documents() {
        for stable in document.summary().stable_ids() {
            let insertion = match insertion_kind(stable) {
                Ok(Some(insertion)) => insertion,
                Ok(None) => continue,
                Err(()) => {
                    return Err(AuthoringEditError::UnsupportedStableId {
                        document: document.key().clone(),
                    });
                }
            };
            let ordinal_key = (
                document.key().clone(),
                stable.enclosing_block().as_str().to_owned(),
                stable.kind(),
            );
            let ordinal = ordinals.entry(ordinal_key).or_insert(0);
            *ordinal = ordinal.saturating_add(1);
            if !select(stable) {
                continue;
            }
            candidates.push((document, stable, insertion, *ordinal));
        }
    }

    let mut edits = Vec::new();
    for (document, stable, insertion, ordinal) in candidates {
        let Some(span) = stable.insertion_span() else {
            return Err(AuthoringEditError::MissingSpan {
                document: document.key().clone(),
                role: "stable-ID insertion",
            });
        };
        let label = insertion_label(stable, ordinal);
        let kind = source_id_kind(stable.kind());
        let anchor = generated_anchor(
            &mut occupied,
            document.key().as_str(),
            kind,
            stable.span().start.line(),
            stable.span().start.column(),
            &label,
        )
        .ok_or_else(|| AuthoringEditError::AnchorNamespaceExhausted {
            document: document.key().clone(),
        })?;
        let insertion_position = match insertion {
            InsertionKind::FullId => SourcePosition::new(
                stable.span().start.line(),
                stable.span().start.column().checked_add(1).ok_or_else(|| {
                    AuthoringEditError::UnmappableRange {
                        document: document.key().clone(),
                    }
                })?,
            )
            .map_err(|_| AuthoringEditError::UnmappableRange {
                document: document.key().clone(),
            })?,
            InsertionKind::AnchorOnly | InsertionKind::AtAnchor => span.start,
        };
        let replacement = insertion_text(
            document.key(),
            document.source_text(),
            insertion_position,
            insertion,
            &label,
            &anchor,
        )?;
        edits.push(SourceEdit::new(
            document.key().clone(),
            point_range(insertion_position),
            replacement,
        ));
    }
    if edits.is_empty() {
        return Err(AuthoringEditError::NoEdits);
    }
    make_plan(
        snapshot,
        if keys.len() == 1 { all_keys } else { keys },
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

fn insertion_text(
    document: &recite_core::DocumentKey,
    source: &str,
    position: SourcePosition,
    insertion: InsertionKind,
    label: &str,
    anchor: &str,
) -> Result<String, AuthoringEditError> {
    match insertion {
        InsertionKind::AnchorOnly => Ok(anchor.to_owned()),
        InsertionKind::AtAnchor => Ok(format!("@{anchor}")),
        InsertionKind::FullId => {
            let range = point_range(position);
            let (_, offset) = super::validate::byte_offsets(source, range).map_err(|_| {
                AuthoringEditError::UnmappableRange {
                    document: document.clone(),
                }
            })?;
            let next = source[offset..].chars().next();
            let id = format!("{label}@{anchor}");
            Ok(if next.is_none_or(char::is_whitespace) {
                format!(" {id}")
            } else {
                format!(" {id} ")
            })
        }
    }
}
