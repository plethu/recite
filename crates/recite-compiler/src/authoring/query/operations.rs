use std::collections::BTreeSet;

use recite_core::{Diagnostic, DocumentKey, SourcePosition};

use super::super::snapshot::AuthoringSnapshot;
use super::symbols::{contains, symbol_locations};
use super::types::{
    CompletionItem, HoverInfo, NavigationResult, QueryResult, SemanticFact, SymbolIdentity,
    SymbolKind, SymbolLocation, SymbolQueryOptions, SymbolRole,
};

impl AuthoringSnapshot {
    /// Returns all diagnostics in deterministic document and source order.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        self.diagnostic_values()
    }

    /// Returns diagnostics for one document, preserving partial recovery.
    #[must_use]
    pub fn document_diagnostics(&self, key: &DocumentKey) -> QueryResult<&[Diagnostic]> {
        let Some(document) = self.document(key) else {
            return QueryResult::NoMatch;
        };
        if document.participation() == crate::ValidationParticipation::all_complete() {
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
        if document.participation() == crate::ValidationParticipation::all_complete() {
            QueryResult::Ready(locations)
        } else {
            QueryResult::Partial(locations)
        }
    }

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
            return QueryResult::Unavailable;
        }
        let mut items = Vec::new();
        let mut seen = BTreeSet::new();
        let mut incomplete_targets = false;
        for target in self.documents() {
            if reference
                .file()
                .is_some_and(|file| file != target.key().as_str())
            {
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
                let declaration = SymbolLocation {
                    document: target.key().clone(),
                    identity: SymbolIdentity::Block(block.id().clone()),
                    kind: SymbolKind::Block,
                    role: SymbolRole::Definition,
                    span: block.id_span().clone(),
                };
                items.push(CompletionItem {
                    identity: declaration.identity.clone(),
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
            QueryResult::Unavailable
        } else if incomplete_targets {
            QueryResult::Partial(items)
        } else if items.is_empty() {
            QueryResult::NoMatch
        } else {
            QueryResult::Ready(items)
        }
    }

    /// Returns typed facts for the symbol at a source position.
    #[must_use]
    pub fn hover(&self, key: &DocumentKey, position: SourcePosition) -> QueryResult<HoverInfo> {
        let Some(document) = self.document(key) else {
            return QueryResult::NoMatch;
        };
        let (QueryResult::Partial(locations) | QueryResult::Ready(locations)) =
            self.symbols(key, SymbolQueryOptions::default())
        else {
            return QueryResult::NoMatch;
        };
        let Some(location) = locations
            .into_iter()
            .find(|location| contains(&location.span, position))
        else {
            return QueryResult::NoMatch;
        };
        let facts = match location.role {
            SymbolRole::Definition => vec![SemanticFact::Definition],
            SymbolRole::Reference => vec![SemanticFact::Reference],
            SymbolRole::Annotation => document
                .summary()
                .metadata()
                .iter()
                .find(|metadata| {
                    metadata
                        .key_span()
                        .is_some_and(|span| span == &location.span)
                })
                .map(|metadata| vec![SemanticFact::MetadataValue(metadata.value().clone())])
                .unwrap_or_default(),
            SymbolRole::Invocation => match &location.identity {
                SymbolIdentity::Function(name) => document
                    .summary()
                    .condition_functions()
                    .iter()
                    .chain(document.summary().effect_functions())
                    .find(|function| function.name() == name && function.span() == &location.span)
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
        let info = HoverInfo { location, facts };
        if document.participation() == crate::ValidationParticipation::all_complete() {
            QueryResult::Ready(info)
        } else {
            QueryResult::Partial(info)
        }
    }

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
            .find(|symbol| contains(&symbol.span, position))
        else {
            return QueryResult::NoMatch;
        };
        let SymbolIdentity::Block(block_id) = &symbol.identity else {
            return QueryResult::Ready(NavigationResult::Unique(symbol));
        };
        let qualified_file = document
            .summary()
            .block_references()
            .iter()
            .find(|reference| {
                reference
                    .block_id_span()
                    .is_some_and(|span| span == &symbol.span)
            })
            .and_then(|reference| reference.file());
        let mut declarations = Vec::new();
        let mut incomplete_targets = false;
        for target in self.documents() {
            if qualified_file.is_some_and(|file| file != target.key().as_str()) {
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
                    .map(|block| SymbolLocation {
                        document: target.key().clone(),
                        identity: SymbolIdentity::Block(block.id().clone()),
                        kind: SymbolKind::Block,
                        role: SymbolRole::Definition,
                        span: block.id_span().clone(),
                    }),
            );
        }
        let result = match declarations.as_slice() {
            [] => NavigationResult::Missing,
            [declaration] => NavigationResult::Unique(declaration.clone()),
            _ => NavigationResult::Ambiguous(declarations),
        };
        if incomplete_targets {
            return QueryResult::Unavailable;
        }
        if document.participation().block_references().is_complete() {
            QueryResult::Ready(result)
        } else {
            QueryResult::Partial(result)
        }
    }
}
