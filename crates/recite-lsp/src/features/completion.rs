use lsp_types::{CompletionItem, CompletionItemKind, CompletionResponse, Documentation, Position};
use recite_compiler::{
    AuthoringSnapshot, CompletionCandidate, CompletionCandidateDetail, CompletionCandidateKind,
    CompletionSiteKind, QueryResult, SchemaSummary, SymbolIdentity, SymbolQueryOptions, SymbolRole,
};
use recite_core::DocumentKey;
use recite_ui::{MsgId, UiCatalog};

use crate::position::lsp_position_to_source;

pub(super) fn completion(
    text: &str,
    position: Position,
    key: Option<&DocumentKey>,
    snapshot: &AuthoringSnapshot,
    schema: Option<&SchemaSummary>,
    catalog: &UiCatalog,
) -> Option<CompletionResponse> {
    let key = key?;
    let source_position = lsp_position_to_source(text, position)?;
    let result = snapshot.complete(key, source_position);
    let candidates = match result {
        QueryResult::Ready(candidates)
        | QueryResult::Partial {
            value: candidates, ..
        } => candidates,
        QueryResult::NoMatch | QueryResult::Unavailable(_) => return None,
        _ => return None,
    };
    let mut items = candidates
        .iter()
        .filter_map(|candidate| completion_item(candidate, text, schema, catalog))
        .collect::<Vec<_>>();
    let unqualified_block_site =
        snapshot
            .completion_site(key, source_position)
            .is_some_and(|site| {
                site.kind() == CompletionSiteKind::Block && site.block_target().is_none()
            });
    if unqualified_block_site {
        extend_project_block_items(snapshot, &mut items, catalog);
    }
    items.sort_by(|left, right| left.label.cmp(&right.label));
    items.dedup_by(|left, right| left.label == right.label);
    Some(CompletionResponse::Array(items))
}

fn extend_project_block_items(
    snapshot: &AuthoringSnapshot,
    items: &mut Vec<CompletionItem>,
    catalog: &UiCatalog,
) {
    let result = snapshot.project_symbols(SymbolQueryOptions::new(true));
    let symbols = match result {
        QueryResult::Ready(symbols) | QueryResult::Partial { value: symbols, .. } => symbols,
        QueryResult::NoMatch | QueryResult::Unavailable(_) | _ => return,
    };
    items.extend(symbols.into_iter().filter_map(|symbol| {
        let SymbolIdentity::Block(name) = symbol.identity() else {
            return None;
        };
        (symbol.role() == SymbolRole::Definition).then(|| CompletionItem {
            label: name.as_str().to_owned(),
            kind: Some(CompletionItemKind::REFERENCE),
            detail: Some(catalog.text(MsgId::LspCompletionBlock)),
            ..CompletionItem::default()
        })
    }));
}

fn completion_item(
    candidate: &CompletionCandidate,
    _text: &str,
    schema: Option<&SchemaSummary>,
    catalog: &UiCatalog,
) -> Option<CompletionItem> {
    let mut item = CompletionItem {
        label: candidate.name().to_owned(),
        kind: Some(candidate_kind(candidate.kind())),
        ..CompletionItem::default()
    };
    match candidate.kind() {
        CompletionCandidateKind::Block => {
            item.detail = Some(catalog.text(MsgId::LspCompletionBlock));
        }
        CompletionCandidateKind::Speaker => {
            item.detail = Some(catalog.text(MsgId::LspCompletionSpeaker));
            if let CompletionCandidateDetail::Speaker { display_name } = candidate.detail() {
                item.documentation = display_name.clone().map(Documentation::String);
            }
        }
        CompletionCandidateKind::MetadataKey => {
            item.detail = Some(metadata_key_detail(candidate, catalog));
        }
        CompletionCandidateKind::MetadataValue => {
            if !recite_parser::is_metadata_symbol(candidate.name()) {
                return None;
            }
            item.detail = Some(metadata_value_detail(candidate, catalog));
        }
        CompletionCandidateKind::Condition => {
            let definition = schema?
                .conditions()
                .iter()
                .find(|condition| condition.name() == candidate.name())?;
            item.detail = Some(catalog.format_pairs(
                MsgId::LspCompletionCondition,
                [("returns", super::condition_detail(definition.returns()))],
            ));
            item.documentation = Some(Documentation::String(
                catalog.text(MsgId::LspCompletionConditionDocumentation),
            ));
        }
        CompletionCandidateKind::Effect => {
            let definition = schema?
                .effects()
                .iter()
                .find(|effect| effect.name() == candidate.name())?;
            item.detail = Some(catalog.format_pairs(
                MsgId::LspCompletionEffect,
                [("modes", super::effect_detail(definition.modes()))],
            ));
            item.documentation = Some(Documentation::String(
                catalog.text(MsgId::LspCompletionEffectDocumentation),
            ));
        }
        CompletionCandidateKind::AvailabilityReason => {
            let CompletionCandidateDetail::AvailabilityReason {
                template,
                parameters,
            } = candidate.detail()
            else {
                return None;
            };
            // A reason with parameters is not authorable by the source syntax's
            // shorthand.  Keep the compiler's complete registry available to
            // other typed consumers while preserving the LSP contract.
            if *parameters != 0 {
                return None;
            }
            item.kind = Some(CompletionItemKind::CONSTANT);
            item.detail = Some(catalog.text(MsgId::LspCompletionAvailabilityReason));
            item.documentation = Some(Documentation::String(template.clone()));
        }
        CompletionCandidateKind::ProjectionQuery
        | CompletionCandidateKind::ProjectionProjector
        | CompletionCandidateKind::ProjectionInput
        | CompletionCandidateKind::ProjectionQueryResult
        | CompletionCandidateKind::ProjectionOutput
        | CompletionCandidateKind::ProjectionLabel => {
            // Projection names are only completed in the schema manifest.  A
            // source query should not leak schema-only lexical candidates.
            return None;
        }
        _ => return None,
    }
    Some(item)
}

fn candidate_kind(kind: CompletionCandidateKind) -> CompletionItemKind {
    match kind {
        CompletionCandidateKind::Block | CompletionCandidateKind::ProjectionProjector => {
            CompletionItemKind::REFERENCE
        }
        CompletionCandidateKind::Speaker | CompletionCandidateKind::AvailabilityReason => {
            CompletionItemKind::CONSTANT
        }
        CompletionCandidateKind::MetadataKey => CompletionItemKind::FIELD,
        CompletionCandidateKind::MetadataValue | CompletionCandidateKind::ProjectionOutput => {
            CompletionItemKind::VALUE
        }
        CompletionCandidateKind::Condition
        | CompletionCandidateKind::Effect
        | CompletionCandidateKind::ProjectionQuery => CompletionItemKind::FUNCTION,
        CompletionCandidateKind::ProjectionInput
        | CompletionCandidateKind::ProjectionQueryResult => CompletionItemKind::VARIABLE,
        CompletionCandidateKind::ProjectionLabel => CompletionItemKind::CONSTANT,
        _ => CompletionItemKind::VALUE,
    }
}

fn metadata_key_detail(candidate: &CompletionCandidate, catalog: &UiCatalog) -> String {
    match candidate.detail() {
        CompletionCandidateDetail::Metadata { domain, .. } => domain.as_deref().map_or_else(
            || catalog.text(MsgId::LspCompletionMetadataKey),
            |domain| {
                catalog.format_pairs(
                    MsgId::LspCompletionMetadataKeyWithDomain,
                    [("domain", domain)],
                )
            },
        ),
        _ => catalog.text(MsgId::LspCompletionMetadataKey),
    }
}

fn metadata_value_detail(candidate: &CompletionCandidate, catalog: &UiCatalog) -> String {
    match candidate.detail() {
        CompletionCandidateDetail::Speaker { .. } => catalog.text(MsgId::LspCompletionSpeaker),
        CompletionCandidateDetail::Metadata { domain, .. } => domain.as_deref().map_or_else(
            || catalog.text(MsgId::LspCompletionMetadataKey),
            |domain| catalog.format_pairs(MsgId::LspCompletionMetadataDomain, [("domain", domain)]),
        ),
        CompletionCandidateDetail::SchemaType(type_ref) => super::schema_type_detail(type_ref),
        _ => catalog.text(MsgId::LspCompletionMetadataKey),
    }
}
