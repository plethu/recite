use lsp_types::Hover;
use recite_compiler::{
    AuthoringSnapshot, ClauseKind, FunctionReferenceKind, HoverInfo, SemanticFact, SymbolIdentity,
};
use recite_core::{DocumentKey, ProjectSchema};
use recite_ui::{MsgId, UiArg, UiArgs, UiCatalog};

use crate::position::span_to_range;

use super::position::hover_response;
use super::schema::{metadata_value_hover, schema_candidate_hover, schema_symbol_hover};

pub(super) fn typed_hover(
    key: &DocumentKey,
    snapshot: &AuthoringSnapshot,
    schema: Option<&ProjectSchema>,
    info: &HoverInfo,
    catalog: &UiCatalog,
) -> Option<Hover> {
    let location = info.location();
    let document = snapshot.document(key)?;
    let range = span_to_range(document.source_text(), location.span());
    match location.identity() {
        SymbolIdentity::Block(name) => info
            .facts()
            .iter()
            .any(|fact| matches!(fact, SemanticFact::Definition | SemanticFact::Reference))
            .then(|| {
                hover_response(
                    &catalog.format_pairs(MsgId::LspHoverBlock, [("name", name.as_str())]),
                    range,
                )
            }),
        SymbolIdentity::MetadataKey(name) => {
            let value_hover = metadata_value_hover(schema, info, range, catalog);
            if let Some(value) = value_hover {
                return Some(value);
            }
            if info.metadata_value_detail().is_some()
                || info
                    .facts()
                    .iter()
                    .any(|fact| matches!(fact, SemanticFact::MetadataValueDetail { .. }))
            {
                return None;
            }
            let schema = schema?;
            let definition = schema.metadata.get(name)?;
            let detail = super::super::schema_hover::hover_detail(None, schema, &[], catalog);
            let value = definition.domain.as_ref().map_or_else(
                || {
                    catalog.format_args(
                        MsgId::LspHoverMetadata,
                        &UiArgs::from([
                            ("name".to_owned(), UiArg::from(name.as_str())),
                            ("detail".to_owned(), UiArg::from(detail.clone())),
                        ]),
                    )
                },
                |domain| {
                    catalog.format_args(
                        MsgId::LspHoverMetadataWithDomain,
                        &UiArgs::from([
                            ("name".to_owned(), UiArg::from(name.as_str())),
                            ("domain".to_owned(), UiArg::from(domain.as_str())),
                            ("detail".to_owned(), UiArg::from(detail.clone())),
                        ]),
                    )
                },
            );
            Some(hover_response(&value, range))
        }
        SymbolIdentity::Function(name) => {
            let SemanticFact::Function { kind, .. } = info.facts().first()? else {
                return None;
            };
            let schema = schema?;
            match kind {
                FunctionReferenceKind::BooleanCondition | FunctionReferenceKind::MatchCondition => {
                    let definition = schema.conditions.get(name)?;
                    Some(hover_response(
                        &catalog.format_pairs(
                            MsgId::LspHoverCondition,
                            [(
                                "returns",
                                super::super::condition_detail(&definition.returns),
                            )],
                        ),
                        range,
                    ))
                }
                FunctionReferenceKind::DeferredEffect
                | FunctionReferenceKind::ImmediateEffect
                | FunctionReferenceKind::BlockingEffect => {
                    let definition = schema.effects.get(name)?;
                    Some(hover_response(
                        &catalog.format_pairs(
                            MsgId::LspHoverEffect,
                            [("modes", super::super::effect_detail(&definition.modes))],
                        ),
                        range,
                    ))
                }
                _ => None,
            }
        }
        SymbolIdentity::Clause(kind) => {
            let message = match kind {
                ClauseKind::Requires => MsgId::LspHoverRequires,
                ClauseKind::If => MsgId::LspHoverIf,
                _ => return None,
            };
            Some(hover_response(&catalog.text(message), range))
        }
        SymbolIdentity::Source(_) => None,
        SymbolIdentity::Schema(name) => {
            let schema = schema?;
            if let Some(value) = metadata_value_hover(Some(schema), info, range, catalog) {
                return Some(value);
            }
            info.facts().iter().find_map(|fact| match fact {
                SemanticFact::SchemaCandidate { detail, kind, .. } => Some(schema_candidate_hover(
                    name, range, detail, *kind, schema, catalog,
                )),
                SemanticFact::SchemaSymbol { kind, .. } => {
                    Some(schema_symbol_hover(name, range, kind, schema, catalog))
                }
                SemanticFact::AvailabilityReason { .. } => Some(schema_symbol_hover(
                    name,
                    range,
                    &recite_compiler::SemanticSymbolKind::AvailabilityReason,
                    schema,
                    catalog,
                )),
                _ => None,
            })?
        }
        _ => None,
    }
}
