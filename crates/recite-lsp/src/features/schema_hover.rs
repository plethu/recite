use recite_core::{
    MetadataDomainDefinition, MissingMetadataContextPolicy, ProjectSchema, SchemaTypeRef,
};
use recite_ui::{MsgId, UiArg, UiArgs, UiCatalog};

mod provenance;

use super::context::{SelectorResolution, SelectorSite, resolve_selector};
use provenance::origin_detail;

pub(crate) use provenance::hover_detail;

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
) -> Option<String> {
    let metadata = schema.metadata.get(metadata_key)?;
    if let Some(domain_name) = &metadata.domain {
        return schema_domain_value_hover(schema, domain_name, word, position, catalog);
    }

    match &metadata.type_ref {
        SchemaTypeRef::Registry(name) => {
            let definition = schema.registries.get(name)?;
            if !definition.values.contains(word) {
                return None;
            }
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
                .or_else(|| {
                    Some(catalog.format_args(
                        MsgId::LspHoverRegistryValue,
                        &UiArgs::from([
                            ("word".to_owned(), UiArg::from(word)),
                            ("name".to_owned(), UiArg::from(name)),
                            ("detail".to_owned(), UiArg::from(String::new())),
                        ]),
                    ))
                })
        }
        SchemaTypeRef::Enum(name) => {
            let Some(recite_core::SchemaTypeDefinition::Enum(definition)) = schema.types.get(name)
            else {
                return None;
            };
            definition.values.contains(word).then(|| {
                catalog.format_pairs(MsgId::LspHoverEnumValue, [("word", word), ("name", name)])
            })
        }
        _ => None,
    }
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
