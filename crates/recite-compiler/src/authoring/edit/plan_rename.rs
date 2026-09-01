use recite_core::{BlockId, DocumentKey, SourcePosition, is_valid_source_label};

use super::helpers::{
    incomplete_from_query, make_plan, no_symbol, project_block_definitions,
    require_complete_block_references, source_range,
};
use super::{AuthoringEditError, AuthoringEditOperation, AuthoringEditPlan, SourceEdit};
use crate::authoring::{
    AuthoringSnapshot, NavigationResult, QueryResult, SymbolIdentity, SymbolLocation,
    SymbolQueryOptions, SymbolRole,
};

/// Plans a block rename from the unique block symbol at `position`.
pub fn plan_rename_block(
    snapshot: &AuthoringSnapshot,
    key: &DocumentKey,
    position: SourcePosition,
    new_name: &str,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    let new_name =
        BlockId::new(new_name.to_owned()).map_err(|_| AuthoringEditError::InvalidBlockName {
            name: new_name.to_owned(),
        })?;
    if !valid_block_name(new_name.as_str()) {
        return Err(AuthoringEditError::InvalidBlockName {
            name: new_name.as_str().to_owned(),
        });
    }
    require_complete_block_references(snapshot)?;

    let declaration = unique_navigation(snapshot, key, position)?;
    let SymbolIdentity::Block(old_name) = declaration.identity() else {
        return Err(no_symbol(key, position));
    };
    let target_document = declaration.document().clone();
    if new_name.as_str() == old_name.as_str() {
        return Err(AuthoringEditError::NoEdits);
    }

    let references = match snapshot.references(key, position, SymbolQueryOptions::default()) {
        QueryResult::Ready(locations) => locations,
        QueryResult::Partial { unavailable, .. } | QueryResult::Unavailable(unavailable) => {
            return Err(incomplete_from_query(key, unavailable));
        }
        QueryResult::NoMatch => return Err(no_symbol(key, position)),
    };
    let definitions = references
        .iter()
        .filter(|location| location.role() == SymbolRole::Definition)
        .collect::<Vec<_>>();
    let [definition] = definitions.as_slice() else {
        return if definitions.is_empty() {
            Err(no_symbol(key, position))
        } else {
            Err(AuthoringEditError::AmbiguousBlock {
                block: old_name.clone(),
            })
        };
    };
    if definition.document() != &target_document {
        return Err(no_symbol(key, position));
    }

    let target_symbols = match snapshot.symbols(&target_document, SymbolQueryOptions::default()) {
        QueryResult::Ready(locations) => locations,
        QueryResult::Partial { unavailable, .. } | QueryResult::Unavailable(unavailable) => {
            return Err(incomplete_from_query(&target_document, unavailable));
        }
        QueryResult::NoMatch => {
            return Err(AuthoringEditError::UnknownDocument {
                document: target_document,
            });
        }
    };
    if target_symbols.iter().any(|location| {
        location.role() == SymbolRole::Definition
            && matches!(location.identity(), SymbolIdentity::Block(block) if block == &new_name)
    }) {
        return Err(AuthoringEditError::DestinationCollision {
            document: target_document,
            block: new_name,
        });
    }
    if project_block_definitions(snapshot, key)?
        .into_iter()
        .any(|location| {
            location.document() != &target_document
                && matches!(location.identity(), SymbolIdentity::Block(block) if block == &new_name)
        })
    {
        return Err(AuthoringEditError::DestinationCollision {
            document: target_document,
            block: new_name,
        });
    }

    let mut edits = Vec::with_capacity(references.len());
    let mut edit_keys = Vec::with_capacity(references.len());
    for location in references {
        if !matches!(
            location.role(),
            SymbolRole::Definition | SymbolRole::Reference
        ) {
            return Err(no_symbol(key, position));
        }
        if !matches!(location.identity(), SymbolIdentity::Block(block) if block == old_name) {
            return Err(no_symbol(key, position));
        }
        let span = source_range(location.document(), location.span())?;
        edits.push(SourceEdit::new(
            location.document().clone(),
            span,
            new_name.as_str(),
        ));
        edit_keys.push(location.document().clone());
    }
    make_plan(
        snapshot,
        edit_keys,
        edits,
        AuthoringEditOperation::RenameBlock {
            from: old_name.clone(),
            to: new_name,
        },
    )
}

impl AuthoringSnapshot {
    /// Plans a safe rename of the block symbol at `position`.
    pub fn plan_rename_block(
        &self,
        key: &DocumentKey,
        position: SourcePosition,
        new_name: &str,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_rename_block(self, key, position, new_name)
    }
}

fn unique_navigation(
    snapshot: &AuthoringSnapshot,
    key: &DocumentKey,
    position: SourcePosition,
) -> Result<SymbolLocation, AuthoringEditError> {
    match snapshot.navigate(key, position) {
        QueryResult::Ready(NavigationResult::Unique(location))
            if matches!(location.identity(), SymbolIdentity::Block(_)) =>
        {
            Ok(location)
        }
        QueryResult::Ready(NavigationResult::Ambiguous(_)) => {
            Err(AuthoringEditError::AmbiguousSymbol {
                document: key.clone(),
                line: position.line(),
                column: position.column(),
            })
        }
        QueryResult::Partial { unavailable, .. } | QueryResult::Unavailable(unavailable) => {
            Err(incomplete_from_query(key, unavailable))
        }
        QueryResult::Ready(_) | QueryResult::NoMatch => Err(no_symbol(key, position)),
    }
}

fn valid_block_name(name: &str) -> bool {
    name != recite_core::END_DIVERT_TARGET && !name.contains("::") && is_valid_source_label(name)
}
