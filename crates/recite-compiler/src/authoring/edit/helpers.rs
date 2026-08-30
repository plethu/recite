use recite_core::{DocumentKey, SourcePosition, SourceSpan};

use super::types::{EditPrecondition, SourceFingerprint, SourceRange};
use super::{AuthoringEditError, AuthoringEditOperation, AuthoringEditPlan};
use crate::authoring::{
    AuthoringSnapshot, QueryClass, QueryResult, QueryUnavailableReason, SymbolIdentity,
    SymbolLocation, SymbolQueryOptions,
};

pub(super) fn document<'a>(
    snapshot: &'a AuthoringSnapshot,
    key: &DocumentKey,
) -> Result<&'a super::super::snapshot::DocumentSnapshot, AuthoringEditError> {
    snapshot
        .document(key)
        .ok_or_else(|| AuthoringEditError::UnknownDocument {
            document: key.clone(),
        })
}

pub(super) fn require_complete_block_references(
    snapshot: &AuthoringSnapshot,
) -> Result<(), AuthoringEditError> {
    snapshot
        .documents()
        .iter()
        .find(|document| !document.participation().block_references().is_complete())
        .map_or(Ok(()), |document| {
            Err(AuthoringEditError::Incomplete {
                document: document.key().clone(),
                class: QueryClass::BlockReferences,
            })
        })
}

pub(super) fn precondition(
    snapshot: &AuthoringSnapshot,
    key: &DocumentKey,
) -> Result<EditPrecondition, AuthoringEditError> {
    let document = document(snapshot, key)?;
    Ok(EditPrecondition::new(
        key.clone(),
        document.version(),
        SourceFingerprint::for_source(document.source_text()),
    ))
}

pub(super) fn make_plan(
    snapshot: &AuthoringSnapshot,
    keys: impl IntoIterator<Item = DocumentKey>,
    edits: Vec<super::SourceEdit>,
    operation: AuthoringEditOperation,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    let mut preconditions = Vec::new();
    for key in keys {
        if !preconditions
            .iter()
            .any(|item: &EditPrecondition| item.document() == &key)
        {
            preconditions.push(precondition(snapshot, &key)?);
        }
    }
    AuthoringEditPlan::new(snapshot.generation(), preconditions, edits, operation)
}

pub(super) fn source_range(
    key: &DocumentKey,
    span: &SourceSpan,
) -> Result<SourceRange, AuthoringEditError> {
    if span.file != key.as_str() {
        return Err(AuthoringEditError::UnmappableRange {
            document: key.clone(),
        });
    }
    let Some(end) = span.end else {
        return Err(AuthoringEditError::MissingSpan {
            document: key.clone(),
            role: "source span end",
        });
    };
    let Some(end_column) = end.column().checked_add(1) else {
        return Err(AuthoringEditError::UnmappableRange {
            document: key.clone(),
        });
    };
    let Ok(end) = SourcePosition::new(end.line(), end_column) else {
        return Err(AuthoringEditError::UnmappableRange {
            document: key.clone(),
        });
    };
    let range = SourceRange::new(span.start, end);
    if range.start() > range.end() {
        return Err(AuthoringEditError::UnmappableRange {
            document: key.clone(),
        });
    }
    Ok(range)
}

pub(super) fn point_range(position: SourcePosition) -> SourceRange {
    SourceRange::point(position)
}

pub(super) fn position_in_span(span: &SourceSpan, position: SourcePosition) -> bool {
    span.end
        .is_some_and(|end| span.start <= position && position <= end)
}

pub(super) fn no_symbol(key: &DocumentKey, position: SourcePosition) -> AuthoringEditError {
    AuthoringEditError::NoSymbol {
        document: key.clone(),
        line: position.line(),
        column: position.column(),
    }
}

pub(super) fn incomplete_from_query(
    document: &DocumentKey,
    unavailable: Vec<QueryUnavailableReason>,
) -> AuthoringEditError {
    let class = unavailable
        .into_iter()
        .find_map(|reason| match reason {
            QueryUnavailableReason::Incomplete(class) => Some(class),
            QueryUnavailableReason::MissingMetadataContext
            | QueryUnavailableReason::MalformedMetadataContext
            | QueryUnavailableReason::Unsupported => None,
        })
        .unwrap_or(QueryClass::Diagnostics);
    AuthoringEditError::Incomplete {
        document: document.clone(),
        class,
    }
}

pub(super) fn block_occurrence(
    snapshot: &AuthoringSnapshot,
    key: &DocumentKey,
    position: SourcePosition,
) -> Result<SymbolLocation, AuthoringEditError> {
    let locations = match snapshot.symbols(key, SymbolQueryOptions::default()) {
        QueryResult::Ready(locations) => locations,
        QueryResult::Partial { unavailable, .. } | QueryResult::Unavailable(unavailable) => {
            return Err(incomplete_from_query(key, unavailable));
        }
        QueryResult::NoMatch => return Err(no_symbol(key, position)),
    };
    let matches = locations
        .into_iter()
        .filter(|location| {
            matches!(location.identity(), SymbolIdentity::Block(_))
                && position_in_span(location.span(), position)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [location] => Ok(location.clone()),
        [] => Err(no_symbol(key, position)),
        _ => Err(AuthoringEditError::AmbiguousSymbol {
            document: key.clone(),
            line: position.line(),
            column: position.column(),
        }),
    }
}
