use recite_core::{MetadataDomainDefinition, MissingMetadataContextPolicy, ProjectSchema};
use recite_ui::{MsgId, UiArg, UiArgs, UiCatalog};

/// Renders a compiler-resolved metadata domain value.  Context selection and
/// validity belong to the compiler; this function only reads schema
/// provenance and localises the protocol-facing hover text.
pub(crate) fn schema_domain_value_hover_with_context(
    schema: &ProjectSchema,
    domain_name: &str,
    word: &str,
    context: Option<&str>,
    catalog: &UiCatalog,
) -> Option<String> {
    let domain = schema.metadata_domains.get(domain_name)?;
    match domain {
        MetadataDomainDefinition::Flat(domain) => {
            if !domain.values.contains(word) {
                return None;
            }
            Some(domain_value_text(
                catalog,
                word,
                domain_name,
                String::new(),
                domain.provenance.value_origins.get(word),
            ))
        }
        MetadataDomainDefinition::Contextual(domain) => {
            if let Some(context) = context
                && let Some(values) = domain.values_by_context.get(context)
            {
                if !values.contains(word) {
                    return None;
                }
                let origin = domain
                    .provenance
                    .value_origins
                    .get(context)
                    .and_then(|values| values.get(word));
                let origin = origin.or_else(|| {
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
                return Some(domain_value_text(
                    catalog,
                    word,
                    domain_name,
                    format!(" ({context})"),
                    origin,
                ));
            }
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
                    context.map_or_else(String::new, |context| format!(" ({context})")),
                    fallback.provenance.value_origins.get(word),
                )
            })
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
                UiArg::from(origin.map_or_else(String::new, |origin| {
                    super::provenance::origin_detail(catalog, origin)
                })),
            ),
        ]),
    )
}
