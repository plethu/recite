use recite_core::{ProjectSchema, SchemaTypeRef, SpeakerDefinition};
use recite_ui::{MsgId, UiArg, UiArgs, UiCatalog};

use super::super::context::SelectorSite;
use super::domain::schema_domain_value_hover;
use super::provenance::origin_detail;

pub(crate) enum SchemaValueHover {
    Resolved(String),
    Invalid,
}

pub(crate) struct AuthoringPosition<'a> {
    pub(crate) text: &'a str,
    pub(crate) line_index: usize,
    pub(crate) line: &'a str,
    pub(crate) site: SelectorSite,
}

/// Return provenance hover only for a value valid for the metadata key at the
/// current authoring position. Keeping the key and line context here avoids
/// treating a matching prose token as a schema symbol.
pub(crate) fn schema_value_hover(
    schema: &ProjectSchema,
    metadata_key: &str,
    word: &str,
    position: &AuthoringPosition<'_>,
    catalog: &UiCatalog,
) -> SchemaValueHover {
    if metadata_key == "speaker" {
        return schema
            .speakers
            .get(word)
            .map(|definition| {
                SchemaValueHover::Resolved(speaker_hover_text(word, definition, catalog))
            })
            .unwrap_or(SchemaValueHover::Invalid);
    }
    let Some(metadata) = schema.metadata.get(metadata_key) else {
        return SchemaValueHover::Invalid;
    };
    if let Some(domain_name) = &metadata.domain {
        return schema_domain_value_hover(schema, domain_name, word, position, catalog)
            .map_or(SchemaValueHover::Invalid, SchemaValueHover::Resolved);
    }

    match &metadata.type_ref {
        SchemaTypeRef::Registry(name) => {
            let Some(definition) = schema.registries.get(name) else {
                return SchemaValueHover::Invalid;
            };
            if !definition.values.contains(word) {
                return SchemaValueHover::Invalid;
            }
            SchemaValueHover::Resolved(
                definition
                    .value_origins
                    .get(word)
                    .map(|origin| {
                        catalog.format_args(
                            MsgId::LspHoverRegistryValue,
                            &UiArgs::from([
                                ("word".to_owned(), UiArg::from(word)),
                                ("name".to_owned(), UiArg::from(name)),
                                (
                                    "detail".to_owned(),
                                    UiArg::from(origin_detail(catalog, origin)),
                                ),
                            ]),
                        )
                    })
                    .unwrap_or_else(|| {
                        catalog.format_args(
                            MsgId::LspHoverRegistryValue,
                            &UiArgs::from([
                                ("word".to_owned(), UiArg::from(word)),
                                ("name".to_owned(), UiArg::from(name)),
                                ("detail".to_owned(), UiArg::from(String::new())),
                            ]),
                        )
                    }),
            )
        }
        SchemaTypeRef::Enum(name) => {
            let Some(recite_core::SchemaTypeDefinition::Enum(definition)) = schema.types.get(name)
            else {
                return SchemaValueHover::Invalid;
            };
            definition
                .values
                .contains(word)
                .then(|| {
                    catalog.format_pairs(MsgId::LspHoverEnumValue, [("word", word), ("name", name)])
                })
                .map_or(SchemaValueHover::Invalid, SchemaValueHover::Resolved)
        }
        SchemaTypeRef::Speaker => schema
            .speakers
            .get(word)
            .map(|definition| {
                SchemaValueHover::Resolved(speaker_hover_text(word, definition, catalog))
            })
            .unwrap_or(SchemaValueHover::Invalid),
        _ => SchemaValueHover::Invalid,
    }
}

pub(crate) fn speaker_hover_text(
    word: &str,
    definition: &SpeakerDefinition,
    catalog: &UiCatalog,
) -> String {
    definition.display_name.as_ref().map_or_else(
        || catalog.format_pairs(MsgId::LspHoverSpeaker, [("name", word)]),
        |display_name| {
            catalog.format_pairs(
                MsgId::LspHoverSpeakerWithDisplayName,
                [("name", word), ("display_name", display_name.as_str())],
            )
        },
    )
}
