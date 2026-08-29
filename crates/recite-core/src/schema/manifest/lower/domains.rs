use std::collections::BTreeSet;

use super::super::diagnostics::{DUPLICATE_DEFINITION, MALFORMED_SHAPE};
use super::super::raw::{Named, RawMetadataDomainDefinition};
use super::super::spans::ManifestSpans;
use super::super::validate::{
    PendingDomainReference, duplicate_definition, parse_metadata_context_selector,
    validate_manifest_name,
};
use super::definitions::canonical_string_values_at;
use super::domains_context::lower_missing_context;
use super::domains_provenance::{
    lower_contextual_domain_provenance, lower_flat_domain_provenance, validate_domain_kind_fields,
};
use super::producer::validate_origin_keys;
use crate::Diagnostic;
use crate::DiagnosticArgumentValue;
use crate::schema::{
    ContextualMetadataDomain, FlatMetadataDomain, MetadataDomainDefinition, ProjectSchema,
    schema_diagnostic,
};

#[allow(
    clippy::too_many_arguments,
    reason = "domain lowering carries shared span, validation, and freshness-mode state"
)]
pub(super) fn lower_metadata_domains(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawMetadataDomainDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
    pending_domain_refs: &mut Vec<PendingDomainReference>,
    allow_duplicate_fingerprints: bool,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let entry_path = vec!["metadata_domains".to_owned(), entry.name.clone()];
        let name_span = spans.key_span_at(file, source, &entry_path, &entry.name);
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

        let RawMetadataDomainDefinition {
            kind,
            values,
            selector,
            values_by_context: raw_values_by_context,
            missing_context,
            origin,
            value_origins,
            context_origins,
            producer_fingerprints,
        } = entry.value;
        if !validate_domain_kind_fields(
            diagnostics,
            &kind,
            values.is_some(),
            selector.is_some(),
            raw_values_by_context.is_some(),
            missing_context.is_some(),
            context_origins.is_some(),
            &entry.name,
            name_span.clone(),
        ) {
            continue;
        }
        let definition = match kind.as_str() {
            "flat" => {
                let Some(values) = values.as_ref() else {
                    diagnostics.push(schema_diagnostic(
                        MALFORMED_SHAPE,
                        "diagnostic-schema-001-domain-values",
                        format!("metadata domain '{}' requires values", entry.name),
                        name_span,
                        [(
                            "domain",
                            DiagnosticArgumentValue::String(entry.name.clone()),
                        )],
                    ));
                    continue;
                };
                let values = canonical_string_values_at(
                    file,
                    source,
                    spans,
                    diagnostics,
                    &format!("metadata domain '{}'", entry.name),
                    values,
                    &entry_path,
                );
                let provenance = lower_flat_domain_provenance(
                    spans,
                    file,
                    source,
                    diagnostics,
                    origin,
                    value_origins,
                    producer_fingerprints,
                    &format!("metadata domain '{}'", entry.name),
                    name_span.clone(),
                    allow_duplicate_fingerprints,
                    &entry_path,
                );
                let value_origins_path = {
                    let mut path = entry_path.clone();
                    path.push("value_origins".to_owned());
                    path
                };
                validate_origin_keys(
                    spans,
                    file,
                    source,
                    diagnostics,
                    &format!("metadata domain '{}'", entry.name),
                    &values,
                    provenance.value_origins.keys().cloned(),
                    name_span.clone(),
                    &value_origins_path,
                );
                MetadataDomainDefinition::Flat(FlatMetadataDomain { values, provenance })
            }
            "contextual" => {
                if missing_context.is_none() {
                    diagnostics.push(schema_diagnostic(
                        MALFORMED_SHAPE,
                        "diagnostic-schema-001-domain-missing-context",
                        format!(
                            "metadata domain '{}' requires explicit missing_context in generated JSON",
                            entry.name
                        ),
                        name_span.clone(),
                        [("domain", DiagnosticArgumentValue::String(entry.name.clone()))],
                    ));
                    continue;
                }
                let Some(selector) = selector.as_deref() else {
                    diagnostics.push(schema_diagnostic(
                        MALFORMED_SHAPE,
                        "diagnostic-schema-001-domain-selector-required",
                        format!("metadata domain '{}' requires selector", entry.name),
                        name_span,
                        [(
                            "domain",
                            DiagnosticArgumentValue::String(entry.name.clone()),
                        )],
                    ));
                    continue;
                };
                let mut selector_path = entry_path.clone();
                selector_path.push("selector".to_owned());
                let selector_span = spans.value_span_at(file, source, &selector_path, selector);
                let Some(selector) = parse_metadata_context_selector(selector) else {
                    diagnostics.push(schema_diagnostic(
                        MALFORMED_SHAPE,
                        "diagnostic-schema-001-domain-selector",
                        format!(
                            "metadata domain '{}' uses unsupported selector '{}'",
                            entry.name, selector
                        ),
                        selector_span,
                        [
                            (
                                "domain",
                                DiagnosticArgumentValue::String(entry.name.clone()),
                            ),
                            (
                                "selector",
                                DiagnosticArgumentValue::String(selector.to_owned()),
                            ),
                        ],
                    ));
                    continue;
                };

                let Some(contexts) = raw_values_by_context else {
                    diagnostics.push(schema_diagnostic(
                        MALFORMED_SHAPE,
                        "diagnostic-schema-001-domain-context-values",
                        format!(
                            "metadata domain '{}' requires values_by_context",
                            entry.name
                        ),
                        name_span,
                        [(
                            "domain",
                            DiagnosticArgumentValue::String(entry.name.clone()),
                        )],
                    ));
                    continue;
                };
                let mut values_by_context = std::collections::BTreeMap::new();
                let mut seen_contexts = BTreeSet::new();
                for context in contexts {
                    let mut context_path = entry_path.clone();
                    context_path.extend(["values_by_context".to_owned(), context.name.clone()]);
                    let context_span =
                        spans.key_span_at(file, source, &context_path, &context.name);
                    if !validate_manifest_name(
                        diagnostics,
                        "metadata domain context",
                        &context.name,
                        context_span.clone(),
                    ) {
                        continue;
                    }
                    if !seen_contexts.insert(context.name.clone()) {
                        diagnostics.push(schema_diagnostic(
                            DUPLICATE_DEFINITION,
                            "diagnostic-schema-003-domain-context",
                            format!(
                                "metadata domain '{}' repeats context '{}'",
                                entry.name, context.name
                            ),
                            context_span,
                            [
                                (
                                    "domain",
                                    DiagnosticArgumentValue::String(entry.name.clone()),
                                ),
                                (
                                    "context",
                                    DiagnosticArgumentValue::String(context.name.clone()),
                                ),
                            ],
                        ));
                        continue;
                    }
                    values_by_context.insert(
                        context.name,
                        canonical_string_values_at(
                            file,
                            source,
                            spans,
                            diagnostics,
                            &format!("metadata domain '{}'", entry.name),
                            &context.value,
                            &context_path,
                        ),
                    );
                }

                let missing_context = lower_missing_context(
                    file,
                    source,
                    spans,
                    &entry.name,
                    &entry_path,
                    missing_context,
                    diagnostics,
                    pending_domain_refs,
                );

                let provenance = lower_contextual_domain_provenance(
                    spans,
                    file,
                    source,
                    diagnostics,
                    origin,
                    context_origins,
                    value_origins,
                    producer_fingerprints,
                    &format!("metadata domain '{}'", entry.name),
                    name_span.clone(),
                    allow_duplicate_fingerprints,
                    &entry_path,
                );
                let context_origins_path = {
                    let mut path = entry_path.clone();
                    path.push("context_origins".to_owned());
                    path
                };
                let value_origins_path = {
                    let mut path = entry_path.clone();
                    path.push("value_origins".to_owned());
                    path
                };
                validate_origin_keys(
                    spans,
                    file,
                    source,
                    diagnostics,
                    &format!("metadata domain '{}'", entry.name),
                    &values_by_context.keys().cloned().collect(),
                    provenance.context_origins.keys().cloned(),
                    name_span.clone(),
                    &context_origins_path,
                );
                validate_origin_keys(
                    spans,
                    file,
                    source,
                    diagnostics,
                    &format!("metadata domain '{}' context", entry.name),
                    &values_by_context.keys().cloned().collect(),
                    provenance.value_origins.keys().cloned(),
                    name_span.clone(),
                    &value_origins_path,
                );
                for (context, origins) in &provenance.value_origins {
                    if let Some(values) = values_by_context.get(context) {
                        validate_origin_keys(
                            spans,
                            file,
                            source,
                            diagnostics,
                            &format!("metadata domain '{}' context '{}'", entry.name, context),
                            values,
                            origins.keys().cloned(),
                            name_span.clone(),
                            &{
                                let mut path = value_origins_path.clone();
                                path.push(context.clone());
                                path
                            },
                        );
                    }
                }
                MetadataDomainDefinition::Contextual(ContextualMetadataDomain {
                    selector,
                    values_by_context,
                    missing_context,
                    provenance,
                })
            }
            other => {
                let mut kind_path = entry_path.clone();
                kind_path.push("kind".to_owned());
                let kind_span = spans.value_span_at(file, source, &kind_path, other);
                diagnostics.push(schema_diagnostic(
                    MALFORMED_SHAPE,
                    "diagnostic-schema-001-domain-kind",
                    format!(
                        "metadata domain '{}' uses unsupported kind '{}'",
                        entry.name, other
                    ),
                    kind_span,
                    [
                        (
                            "domain",
                            DiagnosticArgumentValue::String(entry.name.clone()),
                        ),
                        ("kind", DiagnosticArgumentValue::String(other.to_owned())),
                    ],
                ));
                continue;
            }
        };

        schema.metadata_domains.insert(entry.name, definition);
    }
}
