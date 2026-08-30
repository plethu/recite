use recite_core::{DocumentKey, SourcePosition};

use super::helpers::{
    block_occurrence, document, incomplete_from_query, make_plan, no_symbol, source_range,
};
use super::{AuthoringEditError, AuthoringEditOperation, AuthoringEditPlan, SourceEdit};
use crate::authoring::{
    AuthoringSnapshot, BlockTarget, CompletionSiteKind, NavigationResult, QueryResult,
    SymbolIdentity, SymbolQueryOptions, SymbolRole,
};

/// Plans creation of a missing block at the target document's EOF.
pub fn plan_create_block_stub(
    snapshot: &AuthoringSnapshot,
    key: &DocumentKey,
    position: SourcePosition,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    let reference = block_occurrence(snapshot, key, position)?;
    let SymbolIdentity::Block(block) = reference.identity() else {
        return Err(no_symbol(key, position));
    };
    let Some(site) = snapshot.completion_site(key, position) else {
        return Err(no_symbol(key, position));
    };
    if site.kind() != CompletionSiteKind::Block {
        return Err(no_symbol(key, position));
    }
    let references = match snapshot.references(key, position, SymbolQueryOptions::new(false)) {
        QueryResult::Ready(locations) => locations,
        QueryResult::Partial { unavailable, .. } | QueryResult::Unavailable(unavailable) => {
            return Err(incomplete_from_query(key, unavailable));
        }
        QueryResult::NoMatch => return Err(no_symbol(key, position)),
    };
    if !references.iter().any(|location| {
        location.document() == reference.document()
            && location.span() == reference.span()
            && location.role() == SymbolRole::Reference
    }) {
        return Err(no_symbol(key, position));
    }
    let target_key = match site.block_target_resolution() {
        Some(BlockTarget::Local) => key.clone(),
        Some(BlockTarget::Qualified(target)) => target.clone(),
        Some(BlockTarget::InvalidQualified { target }) => {
            return Err(AuthoringEditError::InvalidTargetDocument {
                document: target.clone(),
            });
        }
        None => return Err(no_symbol(key, position)),
    };
    let target = document(snapshot, &target_key).map_err(|error| match error {
        AuthoringEditError::UnknownDocument { .. } => AuthoringEditError::MissingTargetDocument {
            document: target_key.clone(),
        },
        other => other,
    })?;

    match snapshot.navigate(key, position) {
        QueryResult::Ready(NavigationResult::Missing) => {}
        QueryResult::Ready(NavigationResult::Unique(_)) => {
            return Err(AuthoringEditError::TargetAlreadyExists {
                document: target_key,
                block: block.clone(),
            });
        }
        QueryResult::Ready(NavigationResult::Ambiguous(_)) => {
            return Err(AuthoringEditError::AmbiguousBlock {
                block: block.clone(),
            });
        }
        QueryResult::Partial { unavailable, .. } | QueryResult::Unavailable(unavailable) => {
            return Err(incomplete_from_query(key, unavailable));
        }
        QueryResult::Ready(NavigationResult::Unsupported) | QueryResult::NoMatch => {
            return Err(no_symbol(key, position));
        }
    }

    let reference_range = source_range(key, reference.span())?;
    let eof = end_position(target.source_text(), &target_key)?;
    let newline = newline_for(target.source_text());
    let prefix = if target.source_text().is_empty() || target.source_text().ends_with('\n') {
        ""
    } else {
        newline
    };
    let replacement = format!("{prefix}:: {block}{newline}");
    let edits = vec![SourceEdit::new(
        target_key.clone(),
        super::helpers::point_range(eof),
        replacement,
    )];
    make_plan(
        snapshot,
        [key.clone(), target_key.clone()],
        edits,
        AuthoringEditOperation::CreateBlockStub {
            source: key.clone(),
            reference: reference_range,
            target: target_key,
            block: block.clone(),
        },
    )
}

impl AuthoringSnapshot {
    /// Plans a block stub for the missing reference at `position`.
    pub fn plan_create_block_stub(
        &self,
        key: &DocumentKey,
        position: SourcePosition,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_create_block_stub(self, key, position)
    }
}

fn newline_for(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn end_position(
    source: &str,
    document: &DocumentKey,
) -> Result<SourcePosition, AuthoringEditError> {
    let mut line = 1_u32;
    let mut column = 1_u32;
    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\n' {
            line = line.saturating_add(1);
            column = 1;
            index += 1;
            continue;
        }
        if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
            index += 1;
            continue;
        }
        let Some(character) = source[index..].chars().next() else {
            break;
        };
        column = column.saturating_add(1);
        index += character.len_utf8();
    }
    SourcePosition::new(line, column).map_err(|_| AuthoringEditError::UnmappableRange {
        document: document.clone(),
    })
}
