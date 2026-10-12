use recite_core::{BlockId, DocumentKey, SourceSpan};

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
        self.collect_blocks(false, block_symbol)
    }

    /// Recoverable block names borrowed from the immutable snapshot, in
    /// document and source order. Incomplete declarations and interruption
    /// follow the same policy as the full block-symbol projection.
    #[must_use]
    pub fn project_block_names(&self) -> QueryResult<Vec<&BlockId>> {
        self.collect_blocks(false, |_, name, _| name)
    }

    /// Complete-project block authority required by edit planning.
    pub(crate) fn project_block_definitions(&self) -> QueryResult<Vec<SymbolLocation>> {
        self.collect_blocks(true, block_symbol)
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

    fn collect_blocks<'a, T>(
        &'a self,
        require_complete: bool,
        project: impl Fn(&'a DocumentKey, &'a BlockId, &'a SourceSpan) -> T,
    ) -> QueryResult<Vec<T>> {
        let mut values = Vec::new();
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
                    values.push(project(document.key(), block.id(), span));
                }
            }
        }
        // Snapshot documents are key-ordered; block summaries retain source
        // order through region assembly. Both projections preserve that order.
        if unavailable.is_empty() {
            QueryResult::Ready(values)
        } else {
            QueryResult::partial(values, unavailable)
        }
    }
}

fn block_symbol(document: &DocumentKey, name: &BlockId, span: &SourceSpan) -> SymbolLocation {
    SymbolLocation {
        document: document.clone(),
        identity: SymbolIdentity::Block(name.clone()),
        kind: SymbolKind::Block,
        role: SymbolRole::Definition,
        span: span.clone(),
    }
}
