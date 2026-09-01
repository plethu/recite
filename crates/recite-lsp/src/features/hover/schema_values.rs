use lsp_types::Hover;
use recite_compiler::{
    CompletionCandidateDetail, CompletionCandidateKind, HoverInfo, MetadataValueDetail,
    SchemaSummary, SemanticFact,
};
use recite_core::SchemaTypeRef;
use recite_ui::{MsgId, UiArg, UiArgs, UiCatalog};

use super::position::hover_response;

pub(super) fn metadata_value_hover(
    schema: Option<&SchemaSummary>,
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
            let definition = schema
                .speakers()
                .iter()
                .find(|speaker| speaker.name() == value)?;
            super::super::schema_hover::speaker_hover_text(value, definition.definition(), catalog)
        }
        MetadataValueDetail::Registry(type_ref) => {
            let SchemaTypeRef::Registry(name) = type_ref else {
                return None;
            };
            let definition = schema
                .registries()
                .iter()
                .find(|registry| registry.name() == name)?;
            let detail = definition
                .value_origins()
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
            schema.types().iter().find(|definition| {
                definition.name() == name
                    && definition
                        .enum_values()
                        .is_some_and(|values| values.contains(value))
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
    schema: &SchemaSummary,
    catalog: &UiCatalog,
) -> Option<Hover> {
    let value = match kind {
        CompletionCandidateKind::Speaker => {
            let definition = schema
                .speakers()
                .iter()
                .find(|speaker| speaker.name() == word)?;
            super::super::schema_hover::speaker_hover_text(word, definition.definition(), catalog)
        }
        CompletionCandidateKind::MetadataValue | CompletionCandidateKind::MetadataKey => {
            return None;
        }
        CompletionCandidateKind::Condition => {
            let definition = schema
                .conditions()
                .iter()
                .find(|condition| condition.name() == word)?;
            catalog.format_pairs(
                MsgId::LspHoverCondition,
                [(
                    "returns",
                    super::super::condition_detail(definition.returns()),
                )],
            )
        }
        CompletionCandidateKind::Effect => {
            let definition = schema
                .effects()
                .iter()
                .find(|effect| effect.name() == word)?;
            catalog.format_pairs(
                MsgId::LspHoverEffect,
                [("modes", super::super::effect_detail(definition.modes()))],
            )
        }
        CompletionCandidateKind::AvailabilityReason => {
            let definition = schema
                .availability_reasons()
                .iter()
                .find(|reason| reason.id().as_str() == word)?;
            let detail =
                super::super::schema_hover::hover_detail(definition.origin(), schema, &[], catalog);
            catalog.format_args(
                MsgId::LspHoverAvailabilityReason,
                &UiArgs::from([
                    ("name".to_owned(), UiArg::from(word)),
                    ("detail".to_owned(), UiArg::from(detail)),
                ]),
            )
        }
        CompletionCandidateKind::ProjectionQuery => {
            let definition = schema
                .projection_queries()
                .iter()
                .find(|query| query.name() == word)?;
            catalog.format_args(
                MsgId::LspHoverProjectionQuery,
                &UiArgs::from([
                    ("name".to_owned(), UiArg::from(word)),
                    (
                        "returns".to_owned(),
                        UiArg::from(super::super::schema_type_detail(definition.returns())),
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
