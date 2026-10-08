use recite_core::{DocumentKey, SourcePosition};

use super::symbols::symbol_at;
use super::types::{
    NavigationResult, QueryClass, QueryResult, QueryUnavailableReason, SymbolIdentity, SymbolKind,
    SymbolLocation, SymbolRole,
};
use crate::authoring::AuthoringQuery;

impl AuthoringQuery<'_> {
    /// Resolves a block reference or declaration to deterministic declarations.
    #[must_use]
    pub fn navigate(
        &self,
        key: &DocumentKey,
        position: SourcePosition,
    ) -> QueryResult<NavigationResult> {
        if self.checkpoint().is_err() {
            return QueryResult::unavailable(QueryUnavailableReason::Interrupted);
        }
        let Some(document) = self.document(key) else {
            return QueryResult::NoMatch;
        };
        let Some(symbol) = symbol_at(key, document, position) else {
            return QueryResult::NoMatch;
        };
        let SymbolIdentity::Block(block_id) = symbol.identity() else {
            return QueryResult::Ready(NavigationResult::Unsupported);
        };
        let reference_origin = symbol.kind() == SymbolKind::BlockReference;
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
            if self.checkpoint().is_err() {
                return QueryResult::unavailable(QueryUnavailableReason::Interrupted);
            }
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
            return QueryResult::unavailable(QueryUnavailableReason::Incomplete(
                QueryClass::BlockDefinitions,
            ));
        }
        if !reference_origin || document.participation().block_references().is_complete() {
            QueryResult::Ready(result)
        } else {
            QueryResult::partial(
                result,
                vec![QueryUnavailableReason::Incomplete(
                    QueryClass::BlockReferences,
                )],
            )
        }
    }
}
