use lsp_types::Hover;
use recite_compiler::{SchemaSummary, SemanticSymbolKind};
use recite_core::{MetadataDomainDefinition, ProjectionOutputTarget};
use recite_ui::{MsgId, UiArg, UiArgs, UiCatalog};

use super::position::hover_response;

pub(super) fn schema_symbol_hover(
    name: &str,
    range: lsp_types::Range,
    kind: &SemanticSymbolKind,
    schema: &SchemaSummary,
    catalog: &UiCatalog,
) -> Option<Hover> {
    let value = match kind {
        SemanticSymbolKind::Speaker => {
            let definition = schema
                .speakers()
                .iter()
                .find(|speaker| speaker.name() == name)?;
            super::super::schema_hover::speaker_hover_text(name, definition.definition(), catalog)
        }
        SemanticSymbolKind::Registry => {
            let definition = schema
                .registries()
                .iter()
                .find(|registry| registry.name() == name)?;
            let detail = super::super::schema_hover::hover_detail(
                definition.origin(),
                schema,
                &definition.definition().producer_fingerprints,
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
            let definition = schema
                .metadata_domains()
                .iter()
                .find(|domain| domain.name() == name)?;
            let (origin, fingerprints) = match definition.definition() {
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
            let definition = schema
                .metadata()
                .iter()
                .find(|metadata| metadata.name() == name)?;
            let detail = super::super::schema_hover::hover_detail(None, schema, &[], catalog);
            definition.domain().map_or_else(
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
                            ("domain".to_owned(), UiArg::from(domain)),
                            ("detail".to_owned(), UiArg::from(detail.clone())),
                        ]),
                    )
                },
            )
        }
        SemanticSymbolKind::AvailabilityReason => {
            let definition = schema
                .availability_reasons()
                .iter()
                .find(|reason| reason.id().as_str() == name)?;
            let detail =
                super::super::schema_hover::hover_detail(definition.origin(), schema, &[], catalog);
            catalog.format_args(
                MsgId::LspHoverAvailabilityReason,
                &UiArgs::from([
                    ("name".to_owned(), UiArg::from(name)),
                    ("detail".to_owned(), UiArg::from(detail)),
                ]),
            )
        }
        SemanticSymbolKind::Condition => {
            let definition = schema
                .conditions()
                .iter()
                .find(|condition| condition.name() == name)?;
            catalog.format_pairs(
                MsgId::LspHoverCondition,
                [(
                    "returns",
                    super::super::condition_detail(definition.returns()),
                )],
            )
        }
        SemanticSymbolKind::Effect => {
            let definition = schema
                .effects()
                .iter()
                .find(|effect| effect.name() == name)?;
            catalog.format_pairs(
                MsgId::LspHoverEffect,
                [("modes", super::super::effect_detail(definition.modes()))],
            )
        }
        SemanticSymbolKind::ProjectionQuery => {
            let definition = schema
                .projection_queries()
                .iter()
                .find(|query| query.name() == name)?;
            catalog.format_args(
                MsgId::LspHoverProjectionQuery,
                &UiArgs::from([
                    ("name".to_owned(), UiArg::from(name)),
                    (
                        "returns".to_owned(),
                        UiArg::from(super::super::schema_type_detail(definition.returns())),
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

fn projection_output_target_detail(target: &ProjectionOutputTarget) -> &'static str {
    match target {
        ProjectionOutputTarget::Candidate => "candidate",
        ProjectionOutputTarget::Event => "event",
        ProjectionOutputTarget::Prompt => "prompt",
        _ => "unknown",
    }
}
