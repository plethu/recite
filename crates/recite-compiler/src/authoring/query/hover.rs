use recite_core::{DocumentKey, SourcePosition};

use super::super::snapshot::AuthoringSnapshot;
use super::context;
use super::symbols::{contains, symbol_locations};
use super::types::{
    HoverInfo, QueryClass, QueryResult, QueryUnavailableReason, SemanticFact, SymbolIdentity,
    SymbolKind, SymbolQueryOptions, SymbolRole,
};

impl AuthoringSnapshot {
    /// Returns typed facts for the symbol at a source position.
    #[must_use]
    pub fn hover(&self, key: &DocumentKey, position: SourcePosition) -> QueryResult<HoverInfo> {
        let Some(document) = self.document(key) else {
            return QueryResult::NoMatch;
        };
        let source_locations = symbol_locations(key, document, SymbolQueryOptions::default());
        let has_source_symbol = source_locations
            .iter()
            .any(|location| contains(location.span(), position));
        if !has_source_symbol && context::at(key, document.source_text(), position).is_some() {
            let completion = self.complete(key, position);
            if let Some((name, span)) = context::token_at(key, document.source_text(), position) {
                let candidate = match &completion {
                    QueryResult::Ready(candidates)
                    | QueryResult::Partial {
                        value: candidates, ..
                    } => candidates.iter().find(|candidate| candidate.name() == name),
                    QueryResult::Unavailable(_) | QueryResult::NoMatch => None,
                };
                if let Some(candidate) = candidate {
                    let location = super::types::SymbolLocation {
                        document: key.clone(),
                        identity: SymbolIdentity::Schema(name),
                        kind: SymbolKind::Schema,
                        role: SymbolRole::Annotation,
                        span,
                    };
                    let info = HoverInfo {
                        location,
                        facts: vec![SemanticFact::SchemaCandidate {
                            name: candidate.name().to_owned(),
                            kind: candidate.kind(),
                            detail: candidate.detail().clone(),
                        }],
                    };
                    return match completion {
                        QueryResult::Partial { unavailable, .. } => QueryResult::Partial {
                            value: info,
                            unavailable,
                        },
                        QueryResult::Ready(_) => QueryResult::Ready(info),
                        QueryResult::Unavailable(reasons) => QueryResult::Unavailable(reasons),
                        QueryResult::NoMatch => QueryResult::NoMatch,
                    };
                }
            }
        }
        let Some(location) = source_locations
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
                        || metadata
                            .value_element_spans()
                            .iter()
                            .any(|span| span == location.span())
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
                | SymbolIdentity::MetadataKey(_)
                | SymbolIdentity::Schema(_) => Vec::new(),
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
            super::types::SymbolKind::Schema => self.schema.is_some(),
        };
        let symbol_kind = location.kind();
        let info = HoverInfo { location, facts };
        if complete {
            QueryResult::Ready(info)
        } else {
            QueryResult::partial(
                info,
                vec![QueryUnavailableReason::Incomplete(match symbol_kind {
                    super::types::SymbolKind::Block => QueryClass::BlockDefinitions,
                    super::types::SymbolKind::BlockReference => QueryClass::BlockReferences,
                    super::types::SymbolKind::StableId => QueryClass::StableIds,
                    super::types::SymbolKind::Metadata => QueryClass::Metadata,
                    super::types::SymbolKind::ConditionFunction => QueryClass::ConditionFunctions,
                    super::types::SymbolKind::EffectFunction => QueryClass::EffectFunctions,
                    super::types::SymbolKind::Schema => QueryClass::Schema,
                })],
            )
        }
    }
}
