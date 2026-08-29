use recite_core::{
    MetadataDomainDefinition, MissingMetadataContextPolicy, ProjectSchema, SchemaTypeRef,
    SpeakerDefinition,
};
use recite_ui::{MsgId, UiArg, UiArgs, UiCatalog};

mod provenance;

use super::context::{SelectorResolution, SelectorSite, resolve_selector};
use provenance::origin_detail;

pub(crate) use provenance::hover_detail;

pub(super) enum SchemaValueHover {
    Resolved(String),
    Invalid,
}

pub(super) struct AuthoringPosition<'a> {
    pub(super) text: &'a str,
    pub(super) line_index: usize,
    pub(super) line: &'a str,
    pub(super) site: SelectorSite,
}

/// Return provenance hover only for a value valid for the metadata key at the
/// current authoring position. Keeping the key and line context here avoids
/// treating a matching prose token as a schema symbol.
pub(super) fn schema_value_hover(
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

pub(super) fn speaker_hover_text(
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

fn schema_domain_value_hover(
    schema: &ProjectSchema,
    domain_name: &str,
    word: &str,
    position: &AuthoringPosition<'_>,
    catalog: &UiCatalog,
) -> Option<String> {
    let domain = schema.metadata_domains.get(domain_name)?;
    match domain {
        MetadataDomainDefinition::Flat(domain) => {
            if !domain.values.contains(word) {
                return None;
            }
            domain
                .provenance
                .value_origins
                .get(word)
                .map(|origin| {
                    catalog.format_args(
                        MsgId::LspHoverDomainValue,
                        &UiArgs::from([
                            ("word".to_owned(), UiArg::from(word)),
                            ("name".to_owned(), UiArg::from(domain_name)),
                            ("context".to_owned(), UiArg::from(String::new())),
                            (
                                "detail".to_owned(),
                                UiArg::from(origin_detail(catalog, origin)),
                            ),
                        ]),
                    )
                })
                .or_else(|| {
                    Some(catalog.format_args(
                        MsgId::LspHoverDomainValue,
                        &UiArgs::from([
                            ("word".to_owned(), UiArg::from(word)),
                            ("name".to_owned(), UiArg::from(domain_name)),
                            ("context".to_owned(), UiArg::from(String::new())),
                            ("detail".to_owned(), UiArg::from(String::new())),
                        ]),
                    ))
                })
        }
        MetadataDomainDefinition::Contextual(domain) => {
            match resolve_selector(
                &domain.selector,
                position.text,
                position.line,
                position.line_index,
                position.site,
            ) {
                SelectorResolution::Value(context) => {
                    let values = if let Some(values) = domain.values_by_context.get(&context) {
                        values
                    } else {
                        let MissingMetadataContextPolicy::Fallback { domain: fallback } =
                            &domain.missing_context
                        else {
                            return None;
                        };
                        let Some(MetadataDomainDefinition::Flat(fallback)) =
                            schema.metadata_domains.get(fallback)
                        else {
                            return None;
                        };
                        if !fallback.values.contains(word) {
                            return None;
                        }
                        return Some(domain_value_text(
                            catalog,
                            word,
                            domain_name,
                            format!(" ({context})"),
                            fallback.provenance.value_origins.get(word),
                        ));
                    };
                    if !values.contains(word) {
                        return None;
                    }
                    let origin = domain
                        .provenance
                        .value_origins
                        .get(&context)
                        .and_then(|values| values.get(word))
                        .or_else(|| {
                            let MissingMetadataContextPolicy::Fallback { domain: fallback } =
                                &domain.missing_context
                            else {
                                return None;
                            };
                            match schema.metadata_domains.get(fallback) {
                                Some(MetadataDomainDefinition::Flat(fallback)) => {
                                    fallback.provenance.value_origins.get(word)
                                }
                                _ => None,
                            }
                        });
                    Some(domain_value_text(
                        catalog,
                        word,
                        domain_name,
                        format!(" ({context})"),
                        origin,
                    ))
                }
                SelectorResolution::Missing => {
                    let MissingMetadataContextPolicy::Fallback { domain: fallback } =
                        &domain.missing_context
                    else {
                        return None;
                    };
                    let Some(MetadataDomainDefinition::Flat(fallback)) =
                        schema.metadata_domains.get(fallback)
                    else {
                        return None;
                    };
                    fallback.values.contains(word).then(|| {
                        domain_value_text(
                            catalog,
                            word,
                            domain_name,
                            String::new(),
                            fallback.provenance.value_origins.get(word),
                        )
                    })
                }
                SelectorResolution::Malformed => None,
            }
        }
    }
}

fn domain_value_text(
    catalog: &UiCatalog,
    word: &str,
    domain_name: &str,
    context: String,
    origin: Option<&recite_core::ProducerOrigin>,
) -> String {
    catalog.format_args(
        MsgId::LspHoverDomainValue,
        &UiArgs::from([
            ("word".to_owned(), UiArg::from(word)),
            ("name".to_owned(), UiArg::from(domain_name)),
            ("context".to_owned(), UiArg::from(context)),
            (
                "detail".to_owned(),
                UiArg::from(
                    origin.map_or_else(String::new, |origin| origin_detail(catalog, origin)),
                ),
            ),
        ]),
    )
}
