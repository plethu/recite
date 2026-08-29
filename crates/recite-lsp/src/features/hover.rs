use lsp_types::{Hover, Position};
use recite_core::{MetadataDomainDefinition, ProjectSchema, ProjectionOutputTarget};
use recite_ui::{MsgId, UiArg, UiArgs, UiCatalog};

use super::{block_names, condition_detail, effect_detail, schema_type_detail};
use crate::workspace::LiveProjectSnapshot;

mod position;

pub(crate) use position::byte_index_for_utf16_character;
use position::{
    MetadataHover, MetadataHoverInput, find_if_range, find_requires_range, hover_response,
    metadata_hover, word_at,
};

pub(super) fn hover(
    text: &str,
    position: Position,
    schema: Option<&ProjectSchema>,
    snapshot: &LiveProjectSnapshot,
    catalog: &UiCatalog,
) -> Option<Hover> {
    let line_index = usize::try_from(position.line).ok()?;
    let line = text.lines().nth(line_index)?;
    let byte_index = byte_index_for_utf16_character(line, position.character)?;
    if let Some(range) = find_requires_range(line, line_index, byte_index) {
        return Some(hover_response(
            &catalog.text(MsgId::LspHoverRequires),
            range,
        ));
    }
    if let Some(range) = find_if_range(line, line_index, byte_index) {
        return Some(hover_response(&catalog.text(MsgId::LspHoverIf), range));
    }

    let (word, range) = word_at(line, line_index, byte_index)?;
    if let Some(schema) = schema {
        match metadata_hover(MetadataHoverInput {
            text,
            line,
            line_index,
            byte_index,
            word,
            range,
            schema,
            catalog,
        }) {
            MetadataHover::Resolved(value) => return Some(value),
            MetadataHover::Invalid => return None,
            MetadataHover::NotMetadataPosition => {}
        }
        if let Some(definition) = schema.speakers.get(word) {
            let value = super::schema_hover::speaker_hover_text(word, definition, catalog);
            return Some(hover_response(&value, range));
        }
        if let Some(definition) = schema.registries.get(word) {
            let detail = super::schema_hover::hover_detail(
                definition.origin.as_ref(),
                schema,
                &definition.producer_fingerprints,
                catalog,
            );
            let value = catalog.format_args(
                MsgId::LspHoverRegistry,
                &UiArgs::from([
                    ("name".to_owned(), UiArg::from(word)),
                    ("detail".to_owned(), UiArg::from(detail)),
                ]),
            );
            return Some(hover_response(&value, range));
        }
        if let Some(definition) = schema.metadata_domains.get(word) {
            let origin = match definition {
                MetadataDomainDefinition::Flat(domain) => domain.provenance.origin.as_ref(),
                MetadataDomainDefinition::Contextual(domain) => domain.provenance.origin.as_ref(),
            };
            let scoped_fingerprints = match definition {
                MetadataDomainDefinition::Flat(domain) => &domain.provenance.producer_fingerprints,
                MetadataDomainDefinition::Contextual(domain) => {
                    &domain.provenance.producer_fingerprints
                }
            };
            let detail =
                super::schema_hover::hover_detail(origin, schema, scoped_fingerprints, catalog);
            let value = catalog.format_args(
                MsgId::LspHoverMetadataDomain,
                &UiArgs::from([
                    ("name".to_owned(), UiArg::from(word)),
                    ("detail".to_owned(), UiArg::from(detail)),
                ]),
            );
            return Some(hover_response(&value, range));
        }
        if let Some(definition) = schema.metadata.get(word) {
            let detail = super::schema_hover::hover_detail(None, schema, &[], catalog);
            let value = definition.domain.as_ref().map_or_else(
                || {
                    catalog.format_args(
                        MsgId::LspHoverMetadata,
                        &UiArgs::from([
                            ("name".to_owned(), UiArg::from(word)),
                            ("detail".to_owned(), UiArg::from(detail.clone())),
                        ]),
                    )
                },
                |domain| {
                    catalog.format_args(
                        MsgId::LspHoverMetadataWithDomain,
                        &UiArgs::from([
                            ("name".to_owned(), UiArg::from(word)),
                            ("domain".to_owned(), UiArg::from(domain.as_str())),
                            ("detail".to_owned(), UiArg::from(detail.clone())),
                        ]),
                    )
                },
            );
            return Some(hover_response(&value, range));
        }
        if let Some(definition) = schema.availability_reasons.get(word) {
            let detail =
                super::schema_hover::hover_detail(definition.origin.as_ref(), schema, &[], catalog);
            let value = catalog.format_args(
                MsgId::LspHoverAvailabilityReason,
                &UiArgs::from([
                    ("name".to_owned(), UiArg::from(word)),
                    ("detail".to_owned(), UiArg::from(detail)),
                ]),
            );
            return Some(hover_response(&value, range));
        }
        if let Some(definition) = schema.conditions.get(word) {
            return Some(hover_response(
                &catalog.format_pairs(
                    MsgId::LspHoverCondition,
                    [("returns", condition_detail(&definition.returns))],
                ),
                range,
            ));
        }
        if let Some(definition) = schema.effects.get(word) {
            return Some(hover_response(
                &catalog.format_pairs(
                    MsgId::LspHoverEffect,
                    [("modes", effect_detail(&definition.modes))],
                ),
                range,
            ));
        }
        if let Some(definition) = schema.projection_queries.get(word) {
            return Some(hover_response(
                &catalog.format_args(
                    MsgId::LspHoverProjectionQuery,
                    &UiArgs::from([
                        ("name".to_owned(), UiArg::from(word)),
                        (
                            "returns".to_owned(),
                            UiArg::from(schema_type_detail(&definition.returns)),
                        ),
                    ]),
                ),
                range,
            ));
        }
        if let Some(definition) = schema.presentation_projectors.get(word) {
            let args = UiArgs::from([
                ("name".to_owned(), UiArg::from(word)),
                ("inputs".to_owned(), UiArg::from(definition.inputs.len())),
                ("queries".to_owned(), UiArg::from(definition.queries.len())),
                ("outputs".to_owned(), UiArg::from(definition.outputs.len())),
            ]);
            return Some(hover_response(
                &catalog.format_args(MsgId::LspHoverPresentationProjector, &args),
                range,
            ));
        }
        if let Some(value) = projection_output_hover(schema, word, catalog) {
            return Some(hover_response(&value, range));
        }
        if let Some(value) = presentation_label_hover(schema, word, catalog) {
            return Some(hover_response(&value, range));
        }
    }
    if block_names(snapshot).contains(word) {
        return Some(hover_response(
            &catalog.format_pairs(MsgId::LspHoverBlock, [("name", word)]),
            range,
        ));
    }
    None
}

fn projection_output_hover(
    schema: &ProjectSchema,
    word: &str,
    catalog: &UiCatalog,
) -> Option<String> {
    schema
        .presentation_projectors
        .iter()
        .find_map(|(projector_id, projector)| {
            projector.outputs.get(word).map(|output| {
                catalog.format_pairs(
                    MsgId::LspHoverPresentationOutput,
                    vec![
                        ("name", word.to_owned()),
                        ("projector", projector_id.clone()),
                        (
                            "target",
                            projection_output_target_detail(&output.target).to_owned(),
                        ),
                        ("kind", output.kind.to_string()),
                    ],
                )
            })
        })
}

fn presentation_label_hover(
    schema: &ProjectSchema,
    word: &str,
    catalog: &UiCatalog,
) -> Option<String> {
    schema
        .presentation_projectors
        .values()
        .flat_map(|projector| projector.outputs.values())
        .filter_map(|output| output.label.as_ref())
        .find(|label| label.template_id == word)
        .map(|label| {
            catalog.format_args(
                MsgId::LspHoverPresentationLabel,
                &UiArgs::from([
                    ("name".to_owned(), UiArg::from(word)),
                    ("count".to_owned(), UiArg::from(label.args.len())),
                ]),
            )
        })
}

fn projection_output_target_detail(target: &ProjectionOutputTarget) -> &'static str {
    match target {
        ProjectionOutputTarget::Candidate => "candidate",
        ProjectionOutputTarget::Event => "event",
        ProjectionOutputTarget::Prompt => "prompt",
        _ => "unknown",
    }
}
