use recite_core::{Diagnostic, DocumentKey};

use super::super::snapshot::AuthoringSnapshot;
use super::symbols::symbol_locations;
use super::types::{
    QueryClass, QueryResult, QueryUnavailableReason, SymbolIdentity, SymbolKind, SymbolLocation,
    SymbolQueryOptions, SymbolRole,
};
use recite_core::SourcePosition;

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
            QueryResult::partial(
                document.diagnostics(),
                vec![QueryUnavailableReason::Incomplete(QueryClass::Diagnostics)],
            )
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
        let unavailable = incomplete_symbol_classes(document.participation(), options);
        if unavailable.is_empty() {
            QueryResult::Ready(locations)
        } else {
            QueryResult::partial(locations, unavailable)
        }
    }

    /// Returns deterministic symbol occurrences across the effective project.
    #[must_use]
    pub fn project_symbols(&self, options: SymbolQueryOptions) -> QueryResult<Vec<SymbolLocation>> {
        let mut locations = Vec::new();
        let mut unavailable = Vec::new();
        for document in self.documents() {
            locations.extend(symbol_locations(document.key(), document, options));
            unavailable.extend(incomplete_symbol_classes(document.participation(), options));
        }
        if unavailable.is_empty() {
            QueryResult::Ready(locations)
        } else {
            QueryResult::partial(locations, unavailable)
        }
    }

    /// Returns deterministic block references, optionally including declarations.
    #[must_use]
    pub fn references(
        &self,
        key: &DocumentKey,
        position: SourcePosition,
        options: SymbolQueryOptions,
    ) -> QueryResult<Vec<SymbolLocation>> {
        let Some(document) = self.document(key) else {
            return QueryResult::NoMatch;
        };
        let Some(symbol) = symbol_locations(key, document, SymbolQueryOptions::default())
            .into_iter()
            .find(|symbol| super::symbols::contains(symbol.span(), position))
        else {
            return QueryResult::NoMatch;
        };
        let SymbolIdentity::Block(block_id) = symbol.identity() else {
            return QueryResult::unavailable(QueryUnavailableReason::Unsupported);
        };
        let target_key = document
            .summary()
            .block_references()
            .iter()
            .find(|reference| {
                reference
                    .block_id_span()
                    .is_some_and(|span| span == symbol.span())
            })
            .and_then(|reference| reference.file())
            .map_or_else(|| key.as_str().to_owned(), ToOwned::to_owned);
        let mut locations = Vec::new();
        let mut unavailable = Vec::new();
        for target in self.documents() {
            let relevant_references = target.key().as_str() == target_key
                || target
                    .summary()
                    .block_references()
                    .iter()
                    .any(|reference| reference.file().is_some_and(|file| file == target_key));
            if target.key().as_str() == target_key {
                if options.include_declarations()
                    && !target.participation().block_definitions().is_complete()
                {
                    unavailable.push(QueryUnavailableReason::Incomplete(
                        QueryClass::BlockDefinitions,
                    ));
                } else if options.include_declarations() {
                    locations.extend(target.summary().blocks().iter().filter_map(|block| {
                        (block.id() == block_id).then(|| {
                            Some(SymbolLocation {
                                document: target.key().clone(),
                                identity: SymbolIdentity::Block(block.id().clone()),
                                kind: SymbolKind::Block,
                                role: SymbolRole::Definition,
                                span: block.id_span()?.clone(),
                            })
                        })?
                    }));
                }
            }
            if relevant_references && !target.participation().block_references().is_complete() {
                unavailable.push(QueryUnavailableReason::Incomplete(
                    QueryClass::BlockReferences,
                ));
                continue;
            }
            if !relevant_references {
                continue;
            }
            locations.extend(
                target
                    .summary()
                    .block_references()
                    .iter()
                    .filter_map(|reference| {
                        let scope = reference
                            .file()
                            .map_or_else(|| target.key().as_str().to_owned(), ToOwned::to_owned);
                        (scope == target_key && reference.block_id() == block_id).then(|| {
                            Some(SymbolLocation {
                                document: target.key().clone(),
                                identity: SymbolIdentity::Block(block_id.clone()),
                                kind: SymbolKind::BlockReference,
                                role: SymbolRole::Reference,
                                span: reference.block_id_span()?.clone(),
                            })
                        })?
                    }),
            );
        }
        locations.sort_by(|left, right| {
            left.document()
                .cmp(right.document())
                .then_with(|| left.span().start.cmp(&right.span().start))
        });
        unavailable.sort();
        unavailable.dedup();
        if unavailable.is_empty() {
            QueryResult::Ready(locations)
        } else {
            QueryResult::partial(locations, unavailable)
        }
    }
}

fn incomplete_symbol_classes(
    participation: crate::ValidationParticipation,
    options: SymbolQueryOptions,
) -> Vec<QueryUnavailableReason> {
    let mut classes = vec![
        (
            QueryClass::BlockReferences,
            participation.block_references().is_complete(),
        ),
        (
            QueryClass::StableIds,
            participation.stable_ids().is_complete(),
        ),
        (QueryClass::Metadata, participation.metadata().is_complete()),
        (
            QueryClass::ConditionFunctions,
            participation.condition_functions().is_complete(),
        ),
        (
            QueryClass::EffectFunctions,
            participation.effect_functions().is_complete(),
        ),
    ];
    if options.include_declarations() {
        classes.push((
            QueryClass::BlockDefinitions,
            participation.block_definitions().is_complete(),
        ));
    }
    classes
        .into_iter()
        .filter_map(|(class, complete)| {
            (!complete).then_some(QueryUnavailableReason::Incomplete(class))
        })
        .collect()
}
