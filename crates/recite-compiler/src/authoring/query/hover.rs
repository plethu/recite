use recite_core::{DocumentKey, SourcePosition};

use super::super::snapshot::AuthoringSnapshot;
use super::symbols::{contains, symbol_locations};
use super::types::{
    CompletionContext, HoverInfo, QueryResult, SemanticFact, SymbolIdentity, SymbolQueryOptions,
    SymbolRole,
};

impl AuthoringSnapshot {
    /// Returns typed schema facts for a caller-selected hover context.
    #[must_use]
    pub fn hover_context(
        &self,
        key: &DocumentKey,
        position: SourcePosition,
        context: CompletionContext,
    ) -> QueryResult<Vec<SemanticFact>> {
        match self.complete(key, position, context) {
            QueryResult::Ready(candidates) => QueryResult::Ready(
                candidates
                    .into_iter()
                    .map(|candidate| SemanticFact::SchemaCandidate {
                        name: candidate.name().to_owned(),
                        kind: candidate.kind(),
                        detail: candidate.detail().clone(),
                    })
                    .collect(),
            ),
            QueryResult::Partial(candidates) => QueryResult::Partial(
                candidates
                    .into_iter()
                    .map(|candidate| SemanticFact::SchemaCandidate {
                        name: candidate.name().to_owned(),
                        kind: candidate.kind(),
                        detail: candidate.detail().clone(),
                    })
                    .collect(),
            ),
            QueryResult::Unavailable(reason) => QueryResult::Unavailable(reason),
            QueryResult::NoMatch => QueryResult::NoMatch,
        }
    }

    /// Returns typed facts for the symbol at a source position.
    #[must_use]
    pub fn hover(&self, key: &DocumentKey, position: SourcePosition) -> QueryResult<HoverInfo> {
        let Some(document) = self.document(key) else {
            return QueryResult::NoMatch;
        };
        let locations = symbol_locations(key, document, SymbolQueryOptions::default());
        let Some(location) = locations
            .into_iter()
            .find(|location| contains(location.span(), position))
        else {
            return QueryResult::NoMatch;
        };
        let facts = match location.role() {
            SymbolRole::Definition => vec![SemanticFact::Definition],
            SymbolRole::Reference => vec![SemanticFact::Reference],
            SymbolRole::Annotation => document
                .summary()
                .metadata()
                .iter()
                .find(|metadata| {
                    metadata
                        .key_span()
                        .is_some_and(|span| span == location.span())
                        || metadata
                            .value_span()
                            .is_some_and(|span| span == location.span())
                })
                .map(|metadata| vec![SemanticFact::MetadataValue(metadata.value().clone())])
                .unwrap_or_default(),
            SymbolRole::Invocation => match &location.identity() {
                SymbolIdentity::Function(name) => document
                    .summary()
                    .condition_functions()
                    .iter()
                    .chain(document.summary().effect_functions())
                    .find(|function| function.name() == name && function.span() == location.span())
                    .map(|function| {
                        vec![SemanticFact::Function {
                            name: name.clone(),
                            kind: function.kind(),
                            argument_count: function.argument_count(),
                        }]
                    })
                    .unwrap_or_default(),
                SymbolIdentity::Block(_)
                | SymbolIdentity::Source(_)
                | SymbolIdentity::MetadataKey(_) => Vec::new(),
            },
        };
        let complete = match location.kind() {
            super::types::SymbolKind::Block => {
                document.participation().block_definitions().is_complete()
            }
            super::types::SymbolKind::BlockReference => {
                document.participation().block_references().is_complete()
            }
            super::types::SymbolKind::StableId => {
                document.participation().stable_ids().is_complete()
            }
            super::types::SymbolKind::Metadata => document.participation().metadata().is_complete(),
            super::types::SymbolKind::ConditionFunction => {
                document.participation().condition_functions().is_complete()
            }
            super::types::SymbolKind::EffectFunction => {
                document.participation().effect_functions().is_complete()
            }
        };
        let info = HoverInfo { location, facts };
        if complete {
            QueryResult::Ready(info)
        } else {
            QueryResult::Partial(info)
        }
    }
}
