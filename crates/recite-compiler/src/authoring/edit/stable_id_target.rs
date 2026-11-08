use recite_core::{DocumentKey, SourceId, SourcePosition, StableId};

use super::helpers::document;
use super::{AuthoringEditError, SourceRange};
use crate::authoring::{AuthoringQuery, StableIdSummary};

/// Selects the one line or choice header whose stable ID an edit changes.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StableIdTarget {
    /// The header whose `@id` field contains or ends at a source position,
    /// as an editor rename request supplies.
    Position {
        document: DocumentKey,
        position: SourcePosition,
    },
    /// The header whose `@id` field is on a one-based source line.
    Line { document: DocumentKey, line: u32 },
    /// The only header in the project carrying this ID.
    Id(StableId),
}

impl StableIdTarget {
    #[must_use]
    pub const fn position(document: DocumentKey, position: SourcePosition) -> Self {
        Self::Position { document, position }
    }

    #[must_use]
    pub const fn line(document: DocumentKey, line: u32) -> Self {
        Self::Line { document, line }
    }
}

/// The selected header's frozen ID and the source range of its `@id` field.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct StableIdOccurrence {
    document: DocumentKey,
    id: StableId,
    field: SourceRange,
}

impl StableIdOccurrence {
    #[must_use]
    pub fn document(&self) -> &DocumentKey {
        &self.document
    }

    #[must_use]
    pub fn id(&self) -> &StableId {
        &self.id
    }

    /// The complete `@id` field, including its `@`.
    #[must_use]
    pub const fn field_range(&self) -> SourceRange {
        self.field
    }

    /// The ID text after `@`, which an editor offers for renaming.
    #[must_use]
    pub fn id_range(&self) -> SourceRange {
        let start = self.field.start();
        SourcePosition::new(start.line(), start.column().saturating_add(1))
            .map_or(self.field, |start| {
                SourceRange::new(start, self.field.end())
            })
    }
}

pub(super) fn resolve(
    snapshot: &AuthoringQuery<'_>,
    target: &StableIdTarget,
) -> Result<StableIdOccurrence, AuthoringEditError> {
    let scope = match target {
        StableIdTarget::Position { document, .. } | StableIdTarget::Line { document, .. } => {
            Some(document)
        }
        StableIdTarget::Id(_) => None,
    };
    if let Some(key) = scope {
        document(snapshot, key)?;
    }
    let mut found = Vec::new();
    for candidate in snapshot.documents() {
        if scope.is_some_and(|key| key != candidate.key()) {
            continue;
        }
        snapshot.checkpoint()?;
        for stable in candidate.summary().stable_ids() {
            if selected(stable, target) {
                found.push((candidate.key(), stable));
            }
        }
    }
    let (key, stable) = match found.as_slice() {
        [] => return Err(refusal(target, false)),
        [one] => *one,
        [..] => return Err(refusal(target, true)),
    };
    let id = match stable.source_id() {
        SourceId::Frozen(id) => id.clone(),
        SourceId::Malformed { .. } => {
            return Err(AuthoringEditError::UnsupportedStableId {
                document: key.clone(),
            });
        }
        SourceId::Missing => return Err(refusal(target, false)),
    };
    let span = stable
        .source_id_span()
        .ok_or_else(|| AuthoringEditError::MissingSpan {
            document: key.clone(),
            role: "stable-ID field",
        })?;
    Ok(StableIdOccurrence {
        document: key.clone(),
        id,
        field: super::helpers::source_range(key, span)?,
    })
}

fn selected(stable: &StableIdSummary, target: &StableIdTarget) -> bool {
    match target {
        StableIdTarget::Id(id) => stable.source_id().stable_id() == Some(id),
        StableIdTarget::Line { line, .. } => stable
            .source_id_span()
            .is_some_and(|span| span.start.line() == *line),
        StableIdTarget::Position { position, .. } => {
            stable.source_id_span().is_some_and(|span| {
                // Accept the cursor just after the ID, where editors place it
                // after a word selection.
                span.end.is_some_and(|end| {
                    span.start <= *position
                        && position.line() == end.line()
                        && position.column() <= end.column().saturating_add(1)
                })
            })
        }
    }
}

/// No match, or more than one, for a target.
fn refusal(target: &StableIdTarget, ambiguous: bool) -> AuthoringEditError {
    let (document, line, column) = match target {
        StableIdTarget::Id(id) if ambiguous => {
            return AuthoringEditError::AmbiguousStableId { id: id.clone() };
        }
        StableIdTarget::Id(id) => {
            return AuthoringEditError::UnknownStableId {
                id: id.as_str().to_owned(),
            };
        }
        StableIdTarget::Position { document, position } => {
            (document.clone(), position.line(), position.column())
        }
        StableIdTarget::Line { document, line } => (document.clone(), *line, 1),
    };
    if ambiguous {
        AuthoringEditError::AmbiguousSymbol {
            document,
            line,
            column,
        }
    } else {
        AuthoringEditError::NoSymbol {
            document,
            line,
            column,
        }
    }
}

impl AuthoringQuery<'_> {
    /// Resolves the frozen stable ID a rename at `target` would change.
    pub fn stable_id_occurrence(
        &self,
        target: &StableIdTarget,
    ) -> Result<StableIdOccurrence, AuthoringEditError> {
        resolve(self, target)
    }
}
