use crate::authoring::{
    AuthoringQuery, AuthoringSnapshot, CancellationToken, QueryClass, QueryResult,
    QueryUnavailableReason, SymbolIdentity, SymbolKind, SymbolLocation, SymbolRole,
};

impl AuthoringSnapshot {
    /// Recoverable block declarations without materializing unrelated symbols.
    #[must_use]
    pub fn project_block_symbols(&self) -> QueryResult<Vec<SymbolLocation>> {
        self.query(&CancellationToken::new())
            .project_block_symbols()
    }
}
impl AuthoringQuery<'_> {
    /// Recoverable block declarations in deterministic source order.
    #[must_use]
    pub fn project_block_symbols(&self) -> QueryResult<Vec<SymbolLocation>> {
        self.collect_blocks(false)
    }

    /// Complete-project block authority required by edit planning.
    pub(crate) fn project_block_definitions(&self) -> QueryResult<Vec<SymbolLocation>> {
        self.collect_blocks(true)
    }
    /// Check a rename destination without allocating the project's declarations.
    pub(crate) fn rename_destination_exists(
        &self,
        target: &recite_core::DocumentKey,
        name: &recite_core::BlockId,
    ) -> QueryResult<bool> {
        let Some(document) = self.document(target) else {
            return QueryResult::NoMatch;
        };
        // Retain the edit planner's conservative target-document authority.
        let mut unavailable = super::diagnostics::incomplete_symbol_classes(
            document.participation(),
            super::SymbolQueryOptions::default(),
        );
        if !unavailable.is_empty() {
            return QueryResult::partial(false, unavailable);
        }
        if document
            .summary()
            .blocks()
            .iter()
            .any(|block| block.id() == name && block.id_span().is_some())
        {
            return QueryResult::Ready(true);
        }
        if !self.project_complete {
            unavailable.push(QueryUnavailableReason::Incomplete(
                QueryClass::BlockDefinitions,
            ));
        }
        let mut found = false;
        for document in self.documents() {
            if !document.participation().block_definitions().is_complete() {
                unavailable.push(QueryUnavailableReason::Incomplete(
                    QueryClass::BlockDefinitions,
                ));
            }
            for block in document.summary().blocks() {
                if self.checkpoint().is_err() {
                    return QueryResult::unavailable(QueryUnavailableReason::Interrupted);
                }
                found |= block.id() == name && block.id_span().is_some();
            }
        }
        if unavailable.is_empty() {
            QueryResult::Ready(found)
        } else {
            QueryResult::partial(found, unavailable)
        }
    }

    fn collect_blocks(&self, require_complete: bool) -> QueryResult<Vec<SymbolLocation>> {
        let mut locations = Vec::new();
        let mut unavailable = Vec::new();
        if require_complete && !self.project_complete {
            unavailable.push(QueryUnavailableReason::Incomplete(
                QueryClass::BlockDefinitions,
            ));
        }
        for document in self.documents() {
            if self.checkpoint().is_err() {
                return QueryResult::unavailable(QueryUnavailableReason::Interrupted);
            }
            if !document.participation().block_definitions().is_complete() {
                unavailable.push(QueryUnavailableReason::Incomplete(
                    QueryClass::BlockDefinitions,
                ));
                if require_complete {
                    continue;
                }
            }
            for block in document.summary().blocks() {
                if self.checkpoint().is_err() {
                    return QueryResult::unavailable(QueryUnavailableReason::Interrupted);
                }
                if let Some(span) = block.id_span() {
                    locations.push(SymbolLocation {
                        document: document.key().clone(),
                        identity: SymbolIdentity::Block(block.id().clone()),
                        kind: SymbolKind::Block,
                        role: SymbolRole::Definition,
                        span: span.clone(),
                    });
                }
            }
        }
        locations.sort_by(|left, right| {
            left.document()
                .cmp(right.document())
                .then_with(|| left.span().start.cmp(&right.span().start))
        });
        if unavailable.is_empty() {
            QueryResult::Ready(locations)
        } else {
            QueryResult::partial(locations, unavailable)
        }
    }
}
