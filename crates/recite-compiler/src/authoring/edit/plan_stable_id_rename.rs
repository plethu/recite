//! Deliberate stable-ID renames and reassignments.
//!
//! A rename changes identity, so it is one explicit operation over the
//! header and every project catalogue context keyed on the old ID. When the
//! old ID is duplicated, only the selected header changes: the catalogues'
//! translations cannot be attributed to one side and stay with the other.

use std::collections::BTreeSet;

use recite_core::po::PoDocument;
use recite_core::{DocumentKey, StableId, StableIdGenerator};

use super::helpers::make_plan;
use super::stable_id_target::{StableIdOccurrence, StableIdTarget, resolve};
use super::stable_selection::occupied_ids;
use super::{
    AuthoringEditError, AuthoringEditOperation, AuthoringEditPlan, SourceEdit, SourceRange,
    StableIdRefusal,
};
use crate::authoring::{AuthoringQuery, CatalogSnapshot, QueryClass};

/// Characters reserved by catalogue contexts and future pack qualifiers.
const RESERVED: [char; 3] = ['&', ':', '/'];

/// Plans renaming the stable ID selected by `target` to `new_id`.
///
/// `new_id` may be spelled with or without its leading `@`.
pub(super) fn plan_rename_stable_id(
    snapshot: &AuthoringQuery<'_>,
    target: &StableIdTarget,
    new_id: &str,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot.checkpoint()?;
    let new_id = parse_new_id(new_id)?;
    // Keeping the current name is a no-op even when a catalogue is unusable.
    require_complete(snapshot)?;
    if &new_id == resolve(snapshot, target)?.id() {
        return Err(AuthoringEditError::NoEdits);
    }
    let context = RenameContext::new(snapshot, target)?;
    context.check_unused(&new_id)?;
    context.plan(snapshot, new_id)
}

/// Plans giving the header selected by `target` a freshly generated ID.
pub(super) fn plan_reassign_stable_id(
    snapshot: &AuthoringQuery<'_>,
    target: &StableIdTarget,
    ids: &mut StableIdGenerator,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot.checkpoint()?;
    let context = RenameContext::new(snapshot, target)?;
    let new_id = loop {
        snapshot.checkpoint()?;
        let candidate = ids.next_id();
        if !context.is_used(&candidate) {
            break candidate;
        }
    };
    context.plan(snapshot, new_id)
}

fn parse_new_id(spelled: &str) -> Result<StableId, AuthoringEditError> {
    let id = spelled
        .strip_prefix(recite_core::STABLE_ID_FIELD_PREFIX)
        .unwrap_or(spelled);
    let refuse = |reason| AuthoringEditError::InvalidStableId {
        id: id.to_owned(),
        reason,
    };
    if let Some(character) = id.chars().find(|character| RESERVED.contains(character)) {
        return Err(refuse(StableIdRefusal::ReservedCharacter(character)));
    }
    StableId::new(id).map_err(|_| refuse(StableIdRefusal::Malformed))
}

struct RenameContext<'a> {
    occurrence: StableIdOccurrence,
    duplicated: bool,
    occupied: BTreeSet<StableId>,
    catalogs: Vec<(&'a CatalogSnapshot, PoDocument)>,
}

impl<'a> RenameContext<'a> {
    fn new(
        snapshot: &'a AuthoringQuery<'_>,
        target: &StableIdTarget,
    ) -> Result<Self, AuthoringEditError> {
        require_complete(snapshot)?;
        let occurrence = resolve(snapshot, target)?;
        let mut uses = 0_usize;
        for document in snapshot.documents() {
            snapshot.checkpoint()?;
            uses += document
                .summary()
                .stable_ids()
                .iter()
                .filter(|stable| stable.source_id().stable_id() == Some(occurrence.id()))
                .count();
        }
        let occupied = occupied_ids(snapshot)?;
        let mut catalogs = Vec::with_capacity(snapshot.catalogs().len());
        for catalog in snapshot.catalogs() {
            snapshot.checkpoint()?;
            let parsed = PoDocument::parse_with_path(catalog.key().as_str(), catalog.source_text())
                .map_err(|_| AuthoringEditError::MalformedCatalog {
                    document: catalog.key().clone(),
                })?;
            catalogs.push((catalog, parsed));
        }
        Ok(Self {
            occurrence,
            duplicated: uses > 1,
            occupied,
            catalogs,
        })
    }

    fn is_used(&self, id: &StableId) -> bool {
        self.occupied.contains(id)
            || self
                .catalogs
                .iter()
                .any(|(_, parsed)| parsed.has_context_key(id.as_str()))
    }

    /// Refuses an ID used by a header or still keying catalogue entries,
    /// which would otherwise attach orphaned translations to this header.
    fn check_unused(&self, id: &StableId) -> Result<(), AuthoringEditError> {
        if self.occupied.contains(id) {
            return Err(AuthoringEditError::StableIdCollision {
                id: id.clone(),
                document: None,
            });
        }
        if let Some((catalog, _)) = self
            .catalogs
            .iter()
            .find(|(_, parsed)| parsed.has_context_key(id.as_str()))
        {
            return Err(AuthoringEditError::StableIdCollision {
                id: id.clone(),
                document: Some(catalog.key().clone()),
            });
        }
        Ok(())
    }

    fn plan(
        self,
        snapshot: &AuthoringQuery<'_>,
        new_id: StableId,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        let mut edits = vec![SourceEdit::new(
            self.occurrence.document().clone(),
            self.occurrence.field_range(),
            new_id.field().to_string(),
        )];
        if !self.duplicated {
            for (catalog, parsed) in &self.catalogs {
                snapshot.checkpoint()?;
                let replacements = parsed
                    .plan_context_rename(self.occurrence.id().as_str(), new_id.as_str())
                    .map_err(|_| AuthoringEditError::MalformedCatalog {
                        document: catalog.key().clone(),
                    })?;
                for replacement in replacements {
                    edits.push(SourceEdit::new(
                        catalog.key().clone(),
                        catalog_range(catalog, replacement.range())?,
                        replacement.text(),
                    ));
                }
            }
        }
        // Every document and catalogue guards the plan: namespace and
        // catalogue collision checks depend on all of them.
        let keys = snapshot
            .documents()
            .iter()
            .map(|document| document.key().clone())
            .chain(
                self.catalogs
                    .iter()
                    .map(|(catalog, _)| catalog.key().clone()),
            )
            .collect::<Vec<DocumentKey>>();
        let from = self.occurrence.id().clone();
        let operation = if self.duplicated {
            AuthoringEditOperation::ReassignDuplicateStableId { from, to: new_id }
        } else {
            AuthoringEditOperation::RenameStableId { from, to: new_id }
        };
        make_plan(snapshot, keys, edits, operation)
    }
}

/// Renames need the whole project namespace and every header's ID.
fn require_complete(snapshot: &AuthoringQuery<'_>) -> Result<(), AuthoringEditError> {
    if !snapshot.project_complete {
        return Err(AuthoringEditError::IncompleteProject);
    }
    for document in snapshot.documents() {
        snapshot.checkpoint()?;
        if !document.participation().ast_structure().is_complete() {
            return Err(AuthoringEditError::Incomplete {
                document: document.key().clone(),
                class: QueryClass::Diagnostics,
            });
        }
    }
    Ok(())
}

fn catalog_range(
    catalog: &CatalogSnapshot,
    bytes: std::ops::Range<usize>,
) -> Result<SourceRange, AuthoringEditError> {
    let index = catalog.source_index();
    let unmappable = || AuthoringEditError::UnmappableRange {
        document: catalog.key().clone(),
    };
    let start = index
        .position_for_byte_offset(bytes.start)
        .ok_or_else(unmappable)?;
    let end = index
        .position_for_byte_offset(bytes.end)
        .ok_or_else(unmappable)?;
    Ok(SourceRange::new(start, end))
}

impl AuthoringQuery<'_> {
    /// Plans renaming one stable ID and every catalogue context keyed on it.
    pub fn plan_rename_stable_id(
        &self,
        target: &StableIdTarget,
        new_id: &str,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_rename_stable_id(self, target, new_id)
    }

    /// Plans replacing one stable ID with a freshly generated one, as
    /// duplicate-ID repair does.
    pub fn plan_reassign_stable_id(
        &self,
        target: &StableIdTarget,
        ids: &mut StableIdGenerator,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_reassign_stable_id(self, target, ids)
    }
}
