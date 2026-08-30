use recite_core::{Diagnostic, DocumentKey};

use super::super::snapshot::AuthoringSnapshot;
use super::symbols::symbol_locations;
use super::types::{QueryResult, SymbolLocation, SymbolQueryOptions};

impl AuthoringSnapshot {
    /// Returns diagnostics for one document, preserving partial recovery.
    #[must_use]
    pub fn document_diagnostics(&self, key: &DocumentKey) -> QueryResult<&[Diagnostic]> {
        let Some(document) = self.document(key) else {
            return QueryResult::NoMatch;
        };
        if document.participation().ast_structure().is_complete() {
            QueryResult::Ready(document.diagnostics())
        } else {
            QueryResult::Partial(document.diagnostics())
        }
    }

    /// Returns recoverable symbol occurrences for one document.
    #[must_use]
    pub fn symbols(
        &self,
        key: &DocumentKey,
        options: SymbolQueryOptions,
    ) -> QueryResult<Vec<SymbolLocation>> {
        let Some(document) = self.document(key) else {
            return QueryResult::NoMatch;
        };
        let locations = symbol_locations(key, document, options);
        if symbols_complete(document.participation()) {
            QueryResult::Ready(locations)
        } else {
            QueryResult::Partial(locations)
        }
    }

    /// Returns deterministic symbol occurrences across the effective project.
    #[must_use]
    pub fn project_symbols(&self, options: SymbolQueryOptions) -> QueryResult<Vec<SymbolLocation>> {
        let mut locations = Vec::new();
        let mut complete = true;
        for document in self.documents() {
            locations.extend(symbol_locations(document.key(), document, options));
            complete &= symbols_complete(document.participation());
        }
        if complete {
            QueryResult::Ready(locations)
        } else {
            QueryResult::Partial(locations)
        }
    }
}

fn symbols_complete(participation: crate::ValidationParticipation) -> bool {
    participation.block_definitions().is_complete()
        && participation.block_references().is_complete()
        && participation.stable_ids().is_complete()
        && participation.metadata().is_complete()
        && participation.condition_functions().is_complete()
        && participation.effect_functions().is_complete()
}
