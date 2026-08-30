use lsp_types::Hover;
use recite_compiler::{
    CompletionCandidateDetail, CompletionCandidateKind, HoverInfo, MetadataValueDetail,
    SemanticFact, SemanticSymbolKind,
};
use recite_core::{MetadataDomainDefinition, ProjectSchema, ProjectionOutputTarget, SchemaTypeRef};
use recite_ui::{MsgId, UiArg, UiArgs, UiCatalog};

use super::position::hover_response;

pub(super) fn schema_symbol_hover(
    name: &str,
    range: lsp_types::Range,
    kind: &SemanticSymbolKind,
    schema: &ProjectSchema,
    catalog: &UiCatalog,
) -> Option<Hover> {
    let value = match kind {
        SemanticSymbolKind::Speaker => {
            let definition = schema.speakers.get(name)?;
            super::super::schema_hover::speaker_hover_text(name, definition, catalog)
        }
        SemanticSymbolKind::Registry => {
            let definition = schema.registries.get(name)?;
            let detail = super::super::schema_hover::hover_detail(
                definition.origin.as_ref(),
                schema,
                &definition.producer_fingerprints,
                catalog,
            );
            catalog.format_args(
                MsgId::LspHoverRegistry,
                &UiArgs::from([
                    ("name".to_owned(), UiArg::from(name)),
                    ("detail".to_owned(), UiArg::from(detail)),
                ]),
            )
        }
        SemanticSymbolKind::MetadataDomain => {
            let definition = schema.metadata_domains.get(name)?;
            let (origin, fingerprints) = match definition {
                MetadataDomainDefinition::Flat(domain) => (
                    domain.provenance.origin.as_ref(),
                    &domain.provenance.producer_fingerprints,
                ),
                MetadataDomainDefinition::Contextual(domain) => (
                    domain.provenance.origin.as_ref(),
                    &domain.provenance.producer_fingerprints,
                ),
            };
            let detail =
                super::super::schema_hover::hover_detail(origin, schema, fingerprints, catalog);
            catalog.format_args(
                MsgId::LspHoverMetadataDomain,
                &UiArgs::from([
                    ("name".to_owned(), UiArg::from(name)),
                    ("detail".to_owned(), UiArg::from(detail)),
                ]),
            )
        }
        SemanticSymbolKind::Metadata => {
            let definition = schema.metadata.get(name)?;
            let detail = super::super::schema_hover::hover_detail(None, schema, &[], catalog);
            definition.domain.as_ref().map_or_else(
                || {
                    catalog.format_args(
                        MsgId::LspHoverMetadata,
                        &UiArgs::from([
                            ("name".to_owned(), UiArg::from(name)),
                            ("detail".to_owned(), UiArg::from(detail.clone())),
                        ]),
                    )
                },
                |domain| {
                    catalog.format_args(
                        MsgId::LspHoverMetadataWithDomain,
                        &UiArgs::from([
                            ("name".to_owned(), UiArg::from(name)),
                            ("domain".to_owned(), UiArg::from(domain.as_str())),
                            ("detail".to_owned(), UiArg::from(detail.clone())),
                        ]),
                    )
                },
            )
        }
        SemanticSymbolKind::AvailabilityReason => {
            let definition = schema.availability_reasons.get(name)?;
            let detail = super::super::schema_hover::hover_detail(
                definition.origin.as_ref(),
                schema,
                &[],
                catalog,
            );
            catalog.format_args(
                MsgId::LspHoverAvailabilityReason,
                &UiArgs::from([
                    ("name".to_owned(), UiArg::from(name)),
                    ("detail".to_owned(), UiArg::from(detail)),
                ]),
            )
        }
        SemanticSymbolKind::Condition => {
            let definition = schema.conditions.get(name)?;
            catalog.format_pairs(
                MsgId::LspHoverCondition,
                [(
                    "returns",
                    super::super::condition_detail(&definition.returns),
                )],
            )
        }
        SemanticSymbolKind::Effect => {
            let definition = schema.effects.get(name)?;
            catalog.format_pairs(
                MsgId::LspHoverEffect,
                [("modes", super::super::effect_detail(&definition.modes))],
            )
        }
        SemanticSymbolKind::ProjectionQuery => {
            let definition = schema.projection_queries.get(name)?;
            catalog.format_args(
                MsgId::LspHoverProjectionQuery,
                &UiArgs::from([
                    ("name".to_owned(), UiArg::from(name)),
                    (
                        "returns".to_owned(),
                        UiArg::from(super::super::schema_type_detail(&definition.returns)),
                    ),
                ]),
            )
        }
        SemanticSymbolKind::ProjectionProjector {
            inputs,
            queries,
            outputs,
        } => catalog.format_args(
            MsgId::LspHoverPresentationProjector,
            &UiArgs::from([
                ("name".to_owned(), UiArg::from(name)),
                ("inputs".to_owned(), UiArg::from(*inputs)),
                ("queries".to_owned(), UiArg::from(*queries)),
                ("outputs".to_owned(), UiArg::from(*outputs)),
            ]),
        ),
        SemanticSymbolKind::ProjectionOutput {
            projector,
            target,
            kind,
        } => catalog.format_pairs(
            MsgId::LspHoverPresentationOutput,
            vec![
                ("name", name.to_owned()),
                ("projector", projector.clone()),
                ("target", projection_output_target_detail(target).to_owned()),
                ("kind", kind.clone()),
            ],
        ),
        SemanticSymbolKind::ProjectionLabel { arguments } => catalog.format_args(
            MsgId::LspHoverPresentationLabel,
            &UiArgs::from([
                ("name".to_owned(), UiArg::from(name)),
                ("count".to_owned(), UiArg::from(*arguments)),
            ]),
        ),
        _ => return None,
    };
    Some(hover_response(&value, range))
}

pub(super) fn metadata_value_hover(
    schema: Option<&ProjectSchema>,
    info: &HoverInfo,
    range: lsp_types::Range,
    catalog: &UiCatalog,
) -> Option<Hover> {
    let schema = schema?;
    let (value, detail) = info.metadata_value_detail().or_else(|| {
        info.facts().iter().find_map(|fact| match fact {
            SemanticFact::MetadataValueDetail { value, detail, .. } => {
                Some((value.as_str(), detail))
            }
            _ => None,
        })
    })?;
    let rendered = match detail {
        MetadataValueDetail::Invalid => return None,
        MetadataValueDetail::Speaker => {
            let definition = schema.speakers.get(value)?;
            super::super::schema_hover::speaker_hover_text(value, definition, catalog)
        }
        MetadataValueDetail::Registry(type_ref) => {
            let SchemaTypeRef::Registry(name) = type_ref else {
                return None;
            };
            let definition = schema.registries.get(name)?;
            let detail = definition
                .value_origins
                .get(value)
                .map_or_else(String::new, |origin| {
                    super::super::schema_hover::origin_detail(catalog, origin)
                });
            catalog.format_args(
                MsgId::LspHoverRegistryValue,
                &UiArgs::from([
                    ("word".to_owned(), UiArg::from(value)),
                    ("name".to_owned(), UiArg::from(name)),
                    ("detail".to_owned(), UiArg::from(detail)),
                ]),
            )
        }
        MetadataValueDetail::Enum(type_ref) => {
            let SchemaTypeRef::Enum(name) = type_ref else {
                return None;
            };
            schema
                .types
                .get(name)
                .and_then(|definition| match definition {
                    recite_core::SchemaTypeDefinition::Enum(definition)
                        if definition.values.contains(value) =>
                    {
                        Some(())
                    }
                    _ => None,
                })?;
            catalog.format_pairs(MsgId::LspHoverEnumValue, [("word", value), ("name", name)])
        }
        MetadataValueDetail::Domain { name, context } => {
            super::super::schema_hover::schema_domain_value_hover_with_context(
                schema,
                name,
                value,
                context.as_deref(),
                catalog,
            )?
        }
        _ => return None,
    };
    Some(hover_response(&rendered, range))
}

pub(super) fn schema_candidate_hover(
    word: &str,
    range: lsp_types::Range,
    detail: &CompletionCandidateDetail,
    kind: CompletionCandidateKind,
    schema: &ProjectSchema,
    catalog: &UiCatalog,
) -> Option<Hover> {
    let value = match kind {
        CompletionCandidateKind::Speaker => {
            let definition = schema.speakers.get(word)?;
            super::super::schema_hover::speaker_hover_text(word, definition, catalog)
        }
        CompletionCandidateKind::MetadataValue => return None,
        CompletionCandidateKind::Condition => {
            let definition = schema.conditions.get(word)?;
            catalog.format_pairs(
                MsgId::LspHoverCondition,
                [(
                    "returns",
                    super::super::condition_detail(&definition.returns),
                )],
            )
        }
        CompletionCandidateKind::Effect => {
            let definition = schema.effects.get(word)?;
            catalog.format_pairs(
                MsgId::LspHoverEffect,
                [("modes", super::super::effect_detail(&definition.modes))],
            )
        }
        CompletionCandidateKind::AvailabilityReason => {
            let definition = schema.availability_reasons.get(word)?;
            let detail = super::super::schema_hover::hover_detail(
                definition.origin.as_ref(),
                schema,
                &[],
                catalog,
            );
            catalog.format_args(
                MsgId::LspHoverAvailabilityReason,
                &UiArgs::from([
                    ("name".to_owned(), UiArg::from(word)),
                    ("detail".to_owned(), UiArg::from(detail)),
                ]),
            )
        }
        CompletionCandidateKind::MetadataKey => return None,
        CompletionCandidateKind::ProjectionQuery => {
            let definition = schema.projection_queries.get(word)?;
            catalog.format_args(
                MsgId::LspHoverProjectionQuery,
                &UiArgs::from([
                    ("name".to_owned(), UiArg::from(word)),
                    (
                        "returns".to_owned(),
                        UiArg::from(super::super::schema_type_detail(&definition.returns)),
                    ),
                ]),
            )
        }
        CompletionCandidateKind::ProjectionProjector => catalog.format_args(
            MsgId::LspHoverPresentationProjector,
            &UiArgs::from([
                ("name".to_owned(), UiArg::from(word)),
                ("inputs".to_owned(), UiArg::from(0_usize)),
                ("queries".to_owned(), UiArg::from(0_usize)),
                ("outputs".to_owned(), UiArg::from(0_usize)),
            ]),
        ),
        CompletionCandidateKind::ProjectionInput
        | CompletionCandidateKind::ProjectionQueryResult
        | CompletionCandidateKind::ProjectionOutput
        | CompletionCandidateKind::ProjectionLabel => return None,
        _ => return None,
    };
    let _ = detail;
    Some(hover_response(&value, range))
}

fn projection_output_target_detail(target: &ProjectionOutputTarget) -> &'static str {
    match target {
        ProjectionOutputTarget::Candidate => "candidate",
        ProjectionOutputTarget::Event => "event",
        ProjectionOutputTarget::Prompt => "prompt",
        _ => "unknown",
    }
}
