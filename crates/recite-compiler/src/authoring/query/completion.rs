use recite_core::{DocumentKey, SourcePosition};
use std::collections::BTreeSet;

use super::super::snapshot::AuthoringSnapshot;
use super::symbols::contains;
use super::types::{
    CompletionItem, QueryClass, QueryResult, QueryUnavailableReason, SymbolIdentity, SymbolKind,
    SymbolLocation, SymbolRole,
};

impl AuthoringSnapshot {
    /// Returns block declarations relevant to a block-reference position.
    #[must_use]
    pub fn completions(
        &self,
        key: &DocumentKey,
        position: SourcePosition,
    ) -> QueryResult<Vec<CompletionItem>> {
        let Some(document) = self.document(key) else {
            return QueryResult::NoMatch;
        };
        let Some(reference) = document
            .summary()
            .block_references()
            .iter()
            .find(|reference| {
                reference
                    .block_id_span()
                    .is_some_and(|span| contains(span, position))
            })
        else {
            return QueryResult::NoMatch;
        };
        if !document.participation().block_references().is_complete() {
            return QueryResult::unavailable(QueryUnavailableReason::Incomplete(
                QueryClass::BlockReferences,
            ));
        }
        let mut items = Vec::new();
        let mut seen = BTreeSet::new();
        let mut incomplete_targets = false;
        for target in self.documents() {
            let matches_target = reference
                .file()
                .map_or(target.key() == key, |file| file == target.key().as_str());
            if !matches_target {
                continue;
            }
            if !target.participation().block_definitions().is_complete() {
                incomplete_targets = true;
                continue;
            }
            for block in target.summary().blocks() {
                if !seen.insert((target.key().clone(), block.id().clone())) {
                    continue;
                }
                let Some(span) = block.id_span().cloned() else {
                    continue;
                };
                let declaration = SymbolLocation {
                    document: target.key().clone(),
                    identity: SymbolIdentity::Block(block.id().clone()),
                    kind: SymbolKind::Block,
                    role: SymbolRole::Definition,
                    span,
                };
                items.push(CompletionItem {
                    identity: declaration.identity().clone(),
                    kind: SymbolKind::Block,
                    declaration,
                    replace_span: reference
                        .block_id_span()
                        .cloned()
                        .unwrap_or_else(|| reference.span().clone()),
                });
            }
        }
        if incomplete_targets && items.is_empty() {
            QueryResult::unavailable(QueryUnavailableReason::Incomplete(
                QueryClass::BlockDefinitions,
            ))
        } else if incomplete_targets {
            QueryResult::partial(
                items,
                vec![QueryUnavailableReason::Incomplete(
                    QueryClass::BlockDefinitions,
                )],
            )
        } else if items.is_empty() {
            QueryResult::NoMatch
        } else {
            QueryResult::Ready(items)
        }
    }
}
