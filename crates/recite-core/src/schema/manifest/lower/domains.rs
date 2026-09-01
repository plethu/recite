use std::collections::BTreeSet;

use super::super::diagnostics::{DUPLICATE_DEFINITION, MALFORMED_SHAPE};
use super::super::raw::{Named, RawMetadataDomainDefinition};
use super::super::validate::{
    PendingDomainReference, duplicate_definition, parse_metadata_context_selector,
    validate_manifest_name,
};
use super::LoweringContext;
use super::definitions::canonical_string_values_at;
use super::domains_context::lower_missing_context;
use super::domains_provenance::{
    ContextualDomainProvenanceInput, DomainKindFields, FlatDomainProvenanceInput,
    lower_contextual_domain_provenance, lower_flat_domain_provenance, validate_domain_kind_fields,
};
use super::producer::validate_origin_keys;
use crate::DiagnosticArgumentValue;
use crate::schema::{
    ContextualMetadataDomain, FlatMetadataDomain, MetadataDomainDefinition, ProjectSchema,
    schema_diagnostic,
};

pub(super) fn lower_metadata_domains(
    context: &mut LoweringContext<'_>,
    entries: Vec<Named<RawMetadataDomainDefinition>>,
    schema: &mut ProjectSchema,
    pending_domain_refs: &mut Vec<PendingDomainReference>,
    allow_duplicate_fingerprints: bool,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let entry_path = vec!["metadata_domains".to_owned(), entry.name.clone()];
        let name_span = context.key_span_at(&entry_path, &entry.name);
        if !validate_manifest_name(
            context.diagnostics,
            "metadata domain name",
            &entry.name,
            name_span.clone(),
        ) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(
                context.diagnostics,
                "metadata domain",
                &entry.name,
                name_span,
            );
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
            context.diagnostics,
            DomainKindFields {
                kind: &kind,
                has_values: values.is_some(),
                has_selector: selector.is_some(),
                has_values_by_context: raw_values_by_context.is_some(),
                has_missing_context: missing_context.is_some(),
                has_context_origins: context_origins.is_some(),
                owner: &entry.name,
                span: name_span.clone(),
            },
        ) {
            continue;
        }
        let definition = match kind.as_str() {
            "flat" => {
                let Some(values) = values.as_ref() else {
                    context.diagnostics.push(schema_diagnostic(
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
                    context,
                    &format!("metadata domain '{}'", entry.name),
                    values,
                    &entry_path,
                );
                let provenance = lower_flat_domain_provenance(
                    context,
                    FlatDomainProvenanceInput {
                        origin,
                        value_origins,
                        producer_fingerprints,
                        location: super::producer::ProvenanceLocation {
                            owner: &format!("metadata domain '{}'", entry.name),
                            span: name_span.clone(),
                            path: &entry_path,
                        },
                        allow_duplicate_fingerprints,
                    },
                );
                let value_origins_path = {
                    let mut path = entry_path.clone();
                    path.push("value_origins".to_owned());
                    path
                };
                validate_origin_keys(
                    context,
                    &format!("metadata domain '{}'", entry.name),
                    &values,
                    provenance.value_origins.keys().cloned(),
                    &value_origins_path,
                );
                MetadataDomainDefinition::Flat(FlatMetadataDomain { values, provenance })
            }
            "contextual" => {
                if missing_context.is_none() {
                    context.diagnostics.push(schema_diagnostic(
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
                    context.diagnostics.push(schema_diagnostic(
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
                let selector_span = context.value_span_at(&selector_path, selector);
                let Some(selector) = parse_metadata_context_selector(selector) else {
                    context.diagnostics.push(schema_diagnostic(
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
                    context.diagnostics.push(schema_diagnostic(
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
                for context_entry in contexts {
                    let mut context_path = entry_path.clone();
                    context_path
                        .extend(["values_by_context".to_owned(), context_entry.name.clone()]);
                    let context_span = context.key_span_at(&context_path, &context_entry.name);
                    if !validate_manifest_name(
                        context.diagnostics,
                        "metadata domain context",
                        &context_entry.name,
                        context_span.clone(),
                    ) {
                        continue;
                    }
                    if !seen_contexts.insert(context_entry.name.clone()) {
                        context.diagnostics.push(schema_diagnostic(
                            DUPLICATE_DEFINITION,
                            "diagnostic-schema-003-domain-context",
                            format!(
                                "metadata domain '{}' repeats context '{}'",
                                entry.name, context_entry.name
                            ),
                            context_span,
                            [
                                (
                                    "domain",
                                    DiagnosticArgumentValue::String(entry.name.clone()),
                                ),
                                (
                                    "context",
                                    DiagnosticArgumentValue::String(context_entry.name.clone()),
                                ),
                            ],
                        ));
                        continue;
                    }
                    values_by_context.insert(
                        context_entry.name,
                        canonical_string_values_at(
                            context,
                            &format!("metadata domain '{}'", entry.name),
                            &context_entry.value,
                            &context_path,
                        ),
                    );
                }

                let missing_context = lower_missing_context(
                    context,
                    &entry.name,
                    &entry_path,
                    missing_context,
                    pending_domain_refs,
                );

                let provenance = lower_contextual_domain_provenance(
                    context,
                    ContextualDomainProvenanceInput {
                        origin,
                        context_origins,
                        value_origins,
                        producer_fingerprints,
                        location: super::producer::ProvenanceLocation {
                            owner: &format!("metadata domain '{}'", entry.name),
                            span: name_span.clone(),
                            path: &entry_path,
                        },
                        allow_duplicate_fingerprints,
                    },
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
                    context,
                    &format!("metadata domain '{}'", entry.name),
                    &values_by_context.keys().cloned().collect(),
                    provenance.context_origins.keys().cloned(),
                    &context_origins_path,
                );
                validate_origin_keys(
                    context,
                    &format!("metadata domain '{}' context", entry.name),
                    &values_by_context.keys().cloned().collect(),
                    provenance.value_origins.keys().cloned(),
                    &value_origins_path,
                );
                for (context_name, origins) in &provenance.value_origins {
                    if let Some(values) = values_by_context.get(context_name) {
                        validate_origin_keys(
                            context,
                            &format!(
                                "metadata domain '{}' context '{}'",
                                entry.name, context_name
                            ),
                            values,
                            origins.keys().cloned(),
                            &{
                                let mut path = value_origins_path.clone();
                                path.push(context_name.clone());
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
                let kind_span = context.value_span_at(&kind_path, other);
                context.diagnostics.push(schema_diagnostic(
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
