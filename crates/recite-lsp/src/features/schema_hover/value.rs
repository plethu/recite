use std::collections::BTreeSet;

use recite_core::{ProjectSchema, SchemaTypeRef, SpeakerDefinition};
use recite_ui::{MsgId, UiArg, UiArgs, UiCatalog};

use super::super::context::{SelectorResolution, SelectorSite, resolve_selector};
use super::domain::schema_domain_value_hover;
use super::provenance::origin_detail;
use recite_core::{MetadataDomainDefinition, MissingMetadataContextPolicy};

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

pub(crate) fn schema_value_candidates(
    schema: &ProjectSchema,
    metadata_key: &str,
    text: &str,
    line: &str,
    line_index: usize,
    site: SelectorSite,
) -> BTreeSet<String> {
    let Some(metadata) = schema.metadata.get(metadata_key) else {
        return BTreeSet::new();
    };
    if let Some(domain_name) = &metadata.domain {
        return metadata_domain_values(schema, domain_name, text, line, line_index, site);
    }

    match &metadata.type_ref {
        SchemaTypeRef::Speaker => schema.speakers.keys().cloned().collect(),
        SchemaTypeRef::Registry(name) => schema
            .registries
            .get(name)
            .map_or_else(BTreeSet::new, |definition| definition.values.clone()),
        SchemaTypeRef::Enum(name) => match schema.types.get(name) {
            Some(recite_core::SchemaTypeDefinition::Enum(definition)) => definition.values.clone(),
            _ => BTreeSet::new(),
        },
        _ => BTreeSet::new(),
    }
}

/// Return provenance hover only for a value valid for the metadata key at the
/// current authoring position. Keeping the key and line context here avoids
/// treating a matching prose token as a schema symbol.
pub(crate) fn schema_value_hover(
    schema: &ProjectSchema,
    metadata_key: &str,
    word: &str,
    value: &str,
    position: &AuthoringPosition<'_>,
    catalog: &UiCatalog,
) -> SchemaValueHover {
    if metadata_key == "speaker"
        && matches!(position.site, SelectorSite::Block | SelectorSite::Line)
    {
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
    let Some(values) = metadata_symbol_values(value) else {
        return SchemaValueHover::Invalid;
    };
    if !values.iter().any(|value| value == word) {
        return SchemaValueHover::Invalid;
    }
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

fn metadata_symbol_values(value: &str) -> Option<Vec<String>> {
    match recite_parser::parse_metadata_value(value)? {
        recite_core::SourceMetadataValue::Scalar(recite_core::SourceMetadataScalar::Symbol(
            value,
        )) => Some(vec![value]),
        recite_core::SourceMetadataValue::Array(values) => values
            .into_iter()
            .map(|value| match value {
                recite_core::SourceMetadataScalar::Symbol(value) => Some(value),
                _ => None,
            })
            .collect(),
        recite_core::SourceMetadataValue::Scalar(_) => None,
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

fn metadata_domain_values(
    schema: &ProjectSchema,
    domain_name: &str,
    text: &str,
    line: &str,
    line_index: usize,
    site: SelectorSite,
) -> BTreeSet<String> {
    let Some(domain) = schema.metadata_domains.get(domain_name) else {
        return BTreeSet::new();
    };
    match domain {
        MetadataDomainDefinition::Flat(domain) => domain.values.clone(),
        MetadataDomainDefinition::Contextual(domain) => {
            match resolve_selector(&domain.selector, text, line, line_index, site) {
                SelectorResolution::Value(context) => domain
                    .values_by_context
                    .get(context.as_str())
                    .cloned()
                    .unwrap_or_else(|| missing_context_values(schema, &domain.missing_context)),
                SelectorResolution::Missing => {
                    missing_context_values(schema, &domain.missing_context)
                }
                SelectorResolution::Malformed => BTreeSet::new(),
            }
        }
    }
}

fn missing_context_values(
    schema: &ProjectSchema,
    policy: &MissingMetadataContextPolicy,
) -> BTreeSet<String> {
    match policy {
        MissingMetadataContextPolicy::Diagnostic | MissingMetadataContextPolicy::Empty => {
            BTreeSet::new()
        }
        MissingMetadataContextPolicy::Fallback { domain } => {
            match schema.metadata_domains.get(domain) {
                Some(MetadataDomainDefinition::Flat(domain)) => domain.values.clone(),
                _ => BTreeSet::new(),
            }
        }
    }
}
