use recite_core::{DocumentKey, SourcePosition};

use super::super::snapshot::AuthoringSnapshot;
use super::symbols::{contains, symbol_locations};
use super::types::{
    NavigationResult, QueryClass, QueryResult, QueryUnavailableReason, SymbolIdentity, SymbolKind,
    SymbolLocation, SymbolQueryOptions, SymbolRole,
};

impl AuthoringSnapshot {
    /// Resolves a block reference or declaration to deterministic declarations.
    #[must_use]
    pub fn navigate(
        &self,
        key: &DocumentKey,
        position: SourcePosition,
    ) -> QueryResult<NavigationResult> {
        let Some(document) = self.document(key) else {
            return QueryResult::NoMatch;
        };
        let Some(symbol) = symbol_locations(key, document, SymbolQueryOptions::default())
            .into_iter()
            .find(|symbol| contains(symbol.span(), position))
        else {
            return QueryResult::NoMatch;
        };
        let SymbolIdentity::Block(block_id) = symbol.identity() else {
            return QueryResult::Ready(NavigationResult::Unsupported);
        };
        let qualified_file = document
            .summary()
            .block_references()
            .iter()
            .find(|reference| {
                reference
                    .block_id_span()
                    .is_some_and(|span| span == symbol.span())
            })
            .and_then(|reference| reference.file());
        let mut declarations = Vec::new();
        let mut incomplete_targets = false;
        for target in self.documents() {
            let matches_target =
                qualified_file.map_or(target.key() == key, |file| file == target.key().as_str());
            if !matches_target {
                continue;
            }
            if !target.participation().block_definitions().is_complete() {
                incomplete_targets = true;
                continue;
            }
            declarations.extend(
                target
                    .summary()
                    .blocks()
                    .iter()
                    .filter(|block| block.id() == block_id)
                    .filter_map(|block| {
                        Some(SymbolLocation {
                            document: target.key().clone(),
                            identity: SymbolIdentity::Block(block.id().clone()),
                            kind: SymbolKind::Block,
                            role: SymbolRole::Definition,
                            span: block.id_span()?.clone(),
                        })
                    }),
            );
        }
        let result = match declarations.as_slice() {
            [] => NavigationResult::Missing,
            [declaration] => NavigationResult::Unique(declaration.clone()),
            _ => NavigationResult::Ambiguous(declarations),
        };
        if incomplete_targets {
            return QueryResult::Unavailable(QueryUnavailableReason::Incomplete(
                QueryClass::BlockDefinitions,
            ));
        }
        if document.participation().block_references().is_complete() {
            QueryResult::Ready(result)
        } else {
            QueryResult::Partial(result)
        }
    }
}
