use std::collections::BTreeSet;

use recite_core::{
    MetadataContextSelector, MetadataDomainDefinition, MissingMetadataContextPolicy,
    SourceMetadataEntry, SourceMetadataScalar, SourceMetadataValue, SourceSpan,
};

use super::metadata::MetadataValidationContext;
use super::state::Validator;
use crate::diagnostics;

impl<'a> Validator<'a> {
    pub(super) fn validate_metadata_domain_values(
        &mut self,
        entry: &SourceMetadataEntry,
        domain_name: &str,
        values: &[&str],
        span: SourceSpan,
        context: &MetadataValidationContext<'a>,
    ) {
        let Some(schema) = self.schema else {
            return;
        };
        let Some(domain) = schema.metadata_domains.get(domain_name) else {
            return;
        };
        let Some(allowed_values) =
            self.resolve_metadata_domain_values(entry, domain_name, domain, span.clone(), context)
        else {
            return;
        };

        for value in values {
            if !allowed_values.contains(*value) {
                self.diagnostics
                    .push(diagnostics::invalid_metadata_domain_value(
                        &entry.key,
                        domain_name,
                        value,
                        span.clone(),
                    ));
            }
        }
    }

    fn resolve_metadata_domain_values(
        &mut self,
        entry: &SourceMetadataEntry,
        domain_name: &str,
        domain: &'a MetadataDomainDefinition,
        span: SourceSpan,
        context: &MetadataValidationContext<'a>,
    ) -> Option<&'a BTreeSet<String>> {
        let schema = self.schema?;
        match domain {
            MetadataDomainDefinition::Flat(domain) => Some(&domain.values),
            MetadataDomainDefinition::Contextual(domain) => {
                let selector_name = display_metadata_context_selector(&domain.selector);
                let context_value = match &domain.selector {
                    MetadataContextSelector::FieldSpeaker => {
                        context.line_speaker.or(context.block_default_speaker)
                    }
                    MetadataContextSelector::MetadataKey(key) => {
                        let matches = context
                            .metadata
                            .iter()
                            .filter(|metadata_entry| metadata_entry.key == *key)
                            .collect::<Vec<_>>();
                        match matches.as_slice() {
                            [] => None,
                            [selector_entry] => match metadata_scalar_symbol(selector_entry) {
                                Some(value) => Some(value),
                                None => {
                                    let selector_span =
                                        metadata_value_span_from_entry(selector_entry)
                                            .unwrap_or_else(|| span.clone());
                                    self.diagnostics.push(
                                        diagnostics::malformed_metadata_domain_context(
                                            &entry.key,
                                            selector_name.as_str(),
                                            selector_span,
                                        ),
                                    );
                                    return None;
                                }
                            },
                            [_, duplicate_selector, ..] => {
                                let selector_span =
                                    metadata_key_span_from_entry(duplicate_selector)
                                        .unwrap_or_else(|| span.clone());
                                self.diagnostics.push(
                                    diagnostics::malformed_metadata_domain_context(
                                        &entry.key,
                                        selector_name.as_str(),
                                        selector_span,
                                    ),
                                );
                                return None;
                            }
                        }
                    }
                };
                let Some(context_value) = context_value else {
                    return self.missing_context_values(
                        schema,
                        entry,
                        domain_name,
                        selector_name.as_str(),
                        &domain.missing_context,
                        span,
                    );
                };
                let Some(values) = domain.values_by_context.get(context_value) else {
                    return self.missing_context_values(
                        schema,
                        entry,
                        domain_name,
                        selector_name.as_str(),
                        &domain.missing_context,
                        span,
                    );
                };
                Some(values)
            }
        }
    }

    fn missing_context_values(
        &mut self,
        schema: &'a recite_core::ProjectSchema,
        entry: &SourceMetadataEntry,
        domain_name: &str,
        selector: &str,
        policy: &MissingMetadataContextPolicy,
        span: SourceSpan,
    ) -> Option<&'a BTreeSet<String>> {
        match policy {
            MissingMetadataContextPolicy::Diagnostic => {
                self.diagnostics
                    .push(diagnostics::missing_metadata_domain_context(
                        &entry.key,
                        domain_name,
                        selector,
                        span,
                    ));
                None
            }
            MissingMetadataContextPolicy::Empty => Some(empty_string_set()),
            MissingMetadataContextPolicy::Fallback { domain } => {
                let Some(MetadataDomainDefinition::Flat(domain)) =
                    schema.metadata_domains.get(domain)
                else {
                    return None;
                };
                Some(&domain.values)
            }
        }
    }
}

fn metadata_scalar_symbol(entry: &SourceMetadataEntry) -> Option<&str> {
    match &entry.value {
        SourceMetadataValue::Scalar(SourceMetadataScalar::Symbol(value)) => Some(value),
        SourceMetadataValue::Scalar(_) | SourceMetadataValue::Array(_) => None,
    }
}

fn metadata_value_span_from_entry(entry: &SourceMetadataEntry) -> Option<SourceSpan> {
    entry
        .value_span
        .clone()
        .or_else(|| entry.source_span.clone())
}

fn metadata_key_span_from_entry(entry: &SourceMetadataEntry) -> Option<SourceSpan> {
    entry.key_span.clone().or_else(|| entry.source_span.clone())
}

fn display_metadata_context_selector(selector: &MetadataContextSelector) -> String {
    match selector {
        MetadataContextSelector::FieldSpeaker => "field:speaker".to_owned(),
        MetadataContextSelector::MetadataKey(key) => format!("metadata:{key}"),
    }
}

fn empty_string_set() -> &'static BTreeSet<String> {
    use std::sync::OnceLock;

    static EMPTY: OnceLock<BTreeSet<String>> = OnceLock::new();
    EMPTY.get_or_init(BTreeSet::new)
}
