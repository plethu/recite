use std::collections::BTreeSet;

use super::super::diagnostics::{DUPLICATE_DEFINITION, MALFORMED_SHAPE};
use super::super::raw::{Named, RawMetadataDomainDefinition, RawMissingMetadataContext};
use super::super::spans::ManifestSpans;
use super::super::validate::{
    PendingDomainReference, duplicate_definition, parse_metadata_context_selector,
    validate_manifest_name,
};
use super::definitions::canonical_string_values;
use crate::Diagnostic;
use crate::schema::{
    ContextualMetadataDomain, FlatMetadataDomain, MetadataDomainDefinition,
    MissingMetadataContextPolicy, ProjectSchema,
};

pub(super) fn lower_metadata_domains(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawMetadataDomainDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
    pending_domain_refs: &mut Vec<PendingDomainReference>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(
            diagnostics,
            "metadata domain name",
            &entry.name,
            name_span.clone(),
        ) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "metadata domain", &entry.name, name_span);
            continue;
        }

        let definition = match entry.value.kind.as_str() {
            "flat" => {
                let Some(values) = entry.value.values.as_ref() else {
                    diagnostics.push(Diagnostic::error(
                        MALFORMED_SHAPE,
                        format!("metadata domain '{}' requires values", entry.name),
                        name_span,
                    ));
                    continue;
                };
                MetadataDomainDefinition::Flat(FlatMetadataDomain {
                    values: canonical_string_values(
                        file,
                        source,
                        spans,
                        diagnostics,
                        &format!("metadata domain '{}'", entry.name),
                        values,
                    ),
                })
            }
            "contextual" => {
                let Some(selector) = entry.value.selector.as_deref() else {
                    diagnostics.push(Diagnostic::error(
                        MALFORMED_SHAPE,
                        format!("metadata domain '{}' requires selector", entry.name),
                        name_span,
                    ));
                    continue;
                };
                let selector_span = spans.next_value_span(file, source, selector);
                let Some(selector) = parse_metadata_context_selector(selector) else {
                    diagnostics.push(Diagnostic::error(
                        MALFORMED_SHAPE,
                        format!(
                            "metadata domain '{}' uses unsupported selector '{}'",
                            entry.name, selector
                        ),
                        selector_span,
                    ));
                    continue;
                };

                let mut values_by_context = std::collections::BTreeMap::new();
                let mut seen_contexts = BTreeSet::new();
                let Some(contexts) = entry.value.values_by_context else {
                    diagnostics.push(Diagnostic::error(
                        MALFORMED_SHAPE,
                        format!(
                            "metadata domain '{}' requires values_by_context",
                            entry.name
                        ),
                        name_span,
                    ));
                    continue;
                };
                for context in contexts {
                    let context_span = spans.next_key_span(file, source, &context.name);
                    if !validate_manifest_name(
                        diagnostics,
                        "metadata domain context",
                        &context.name,
                        context_span.clone(),
                    ) {
                        continue;
                    }
                    if !seen_contexts.insert(context.name.clone()) {
                        diagnostics.push(Diagnostic::error(
                            DUPLICATE_DEFINITION,
                            format!(
                                "metadata domain '{}' repeats context '{}'",
                                entry.name, context.name
                            ),
                            context_span,
                        ));
                        continue;
                    }
                    values_by_context.insert(
                        context.name,
                        canonical_string_values(
                            file,
                            source,
                            spans,
                            diagnostics,
                            &format!("metadata domain '{}'", entry.name),
                            &context.value,
                        ),
                    );
                }

                let missing_context = lower_missing_context(
                    file,
                    source,
                    spans,
                    &entry.name,
                    entry.value.missing_context,
                    diagnostics,
                    pending_domain_refs,
                );

                MetadataDomainDefinition::Contextual(ContextualMetadataDomain {
                    selector,
                    values_by_context,
                    missing_context,
                })
            }
            other => {
                let kind_span = spans.next_value_span(file, source, other);
                diagnostics.push(Diagnostic::error(
                    MALFORMED_SHAPE,
                    format!(
                        "metadata domain '{}' uses unsupported kind '{}'",
                        entry.name, other
                    ),
                    kind_span,
                ));
                continue;
            }
        };

        schema.metadata_domains.insert(entry.name, definition);
    }
}

fn lower_missing_context(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    owner: &str,
    raw: Option<RawMissingMetadataContext>,
    diagnostics: &mut Vec<Diagnostic>,
    pending_domain_refs: &mut Vec<PendingDomainReference>,
) -> MissingMetadataContextPolicy {
    let Some(raw) = raw else {
        return MissingMetadataContextPolicy::Diagnostic;
    };

    match raw.policy.as_str() {
        "diagnostic" => MissingMetadataContextPolicy::Diagnostic,
        "empty" => MissingMetadataContextPolicy::Empty,
        "fallback" => {
            let Some(domain) = raw.domain else {
                let span = spans.next_value_span(file, source, "fallback");
                diagnostics.push(Diagnostic::error(
                    MALFORMED_SHAPE,
                    format!("metadata domain '{owner}' fallback policy requires domain"),
                    span,
                ));
                return MissingMetadataContextPolicy::Diagnostic;
            };
            let span = spans.next_value_span(file, source, &domain);
            pending_domain_refs.push(PendingDomainReference {
                owner: format!("metadata domain '{owner}' fallback"),
                domain: domain.clone(),
                require_flat: true,
                span,
            });
            MissingMetadataContextPolicy::Fallback { domain }
        }
        other => {
            let span = spans.next_value_span(file, source, other);
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "metadata domain '{owner}' uses unsupported missing_context policy '{other}'"
                ),
                span,
            ));
            MissingMetadataContextPolicy::Diagnostic
        }
    }
}
