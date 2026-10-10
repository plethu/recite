use std::collections::{BTreeMap, BTreeSet};

use super::super::super::diagnostics::{DUPLICATE_DEFINITION, MALFORMED_SHAPE};
use super::super::super::raw::{Named, RawMetadataDomainDefinition};
use super::super::super::validate::{
    PendingDomainReference, parse_metadata_context_selector, validate_manifest_name,
};
use super::super::definitions::canonical_string_values_at;
use super::super::domains_context::lower_missing_context;
use super::super::domains_provenance::{
    ContextualDomainProvenanceInput, lower_contextual_domain_provenance,
};
use super::super::producer::{ProvenanceLocation, validate_origin_keys};
use super::{DomainSite, LoweringContext};
use crate::DiagnosticArgumentValue;
use crate::schema::{
    ContextualMetadataDomain, ContextualMetadataProvenance, MetadataContextSelector,
    schema_diagnostic,
};

pub(super) fn lower(
    context: &mut LoweringContext<'_>,
    site: &DomainSite,
    raw: RawMetadataDomainDefinition,
    pending_domain_refs: &mut Vec<PendingDomainReference>,
    allow_duplicate_fingerprints: bool,
) -> Option<ContextualMetadataDomain> {
    if raw.missing_context.is_none() {
        context.diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-domain-missing-context",
            format!(
                "metadata domain '{}' requires explicit missing_context in generated JSON",
                site.name
            ),
            site.span.clone(),
            [("domain", DiagnosticArgumentValue::String(site.name.clone()))],
        ));
        return None;
    }
    let selector = lower_selector(context, site, raw.selector.as_deref())?;
    let values_by_context = lower_context_values(context, site, raw.values_by_context)?;
    let missing_context = lower_missing_context(
        context,
        &site.name,
        &site.path,
        raw.missing_context,
        pending_domain_refs,
    );
    let provenance = lower_contextual_domain_provenance(
        context,
        ContextualDomainProvenanceInput {
            origin: raw.origin,
            context_origins: raw.context_origins,
            value_origins: raw.value_origins,
            producer_fingerprints: raw.producer_fingerprints,
            location: ProvenanceLocation {
                owner: &format!("metadata domain '{}'", site.name),
                span: site.span.clone(),
                path: &site.path,
            },
            allow_duplicate_fingerprints,
        },
    );
    validate_context_origins(context, site, &values_by_context, &provenance);
    Some(ContextualMetadataDomain {
        selector,
        values_by_context,
        missing_context,
        provenance,
    })
}

fn lower_selector(
    context: &mut LoweringContext<'_>,
    site: &DomainSite,
    selector: Option<&str>,
) -> Option<MetadataContextSelector> {
    let Some(selector) = selector else {
        context.diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-domain-selector-required",
            format!("metadata domain '{}' requires selector", site.name),
            site.span.clone(),
            [("domain", DiagnosticArgumentValue::String(site.name.clone()))],
        ));
        return None;
    };
    let mut selector_path = site.path.clone();
    selector_path.push("selector".to_owned());
    let selector_span = context.value_span_at(&selector_path, selector);
    let Some(selector) = parse_metadata_context_selector(selector) else {
        context.diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-domain-selector",
            format!(
                "metadata domain '{}' uses unsupported selector '{}'",
                site.name, selector
            ),
            selector_span,
            [
                ("domain", DiagnosticArgumentValue::String(site.name.clone())),
                (
                    "selector",
                    DiagnosticArgumentValue::String(selector.to_owned()),
                ),
            ],
        ));
        return None;
    };

    Some(selector)
}

fn lower_context_values(
    context: &mut LoweringContext<'_>,
    site: &DomainSite,
    contexts: Option<Vec<Named<Vec<String>>>>,
) -> Option<BTreeMap<String, BTreeSet<String>>> {
    let Some(contexts) = contexts else {
        context.diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-domain-context-values",
            format!("metadata domain '{}' requires values_by_context", site.name),
            site.span.clone(),
            [("domain", DiagnosticArgumentValue::String(site.name.clone()))],
        ));
        return None;
    };
    let mut values_by_context = std::collections::BTreeMap::new();
    let mut seen_contexts = BTreeSet::new();
    for context_entry in contexts {
        let mut context_path = site.path.clone();
        context_path.extend(["values_by_context".to_owned(), context_entry.name.clone()]);
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
                    site.name, context_entry.name
                ),
                context_span,
                [
                    ("domain", DiagnosticArgumentValue::String(site.name.clone())),
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
                &format!("metadata domain '{}'", site.name),
                &context_entry.value,
                &context_path,
            ),
        );
    }

    Some(values_by_context)
}

fn validate_context_origins(
    context: &mut LoweringContext<'_>,
    site: &DomainSite,
    values_by_context: &BTreeMap<String, BTreeSet<String>>,
    provenance: &ContextualMetadataProvenance,
) {
    let context_origins_path = {
        let mut path = site.path.clone();
        path.push("context_origins".to_owned());
        path
    };
    let value_origins_path = {
        let mut path = site.path.clone();
        path.push("value_origins".to_owned());
        path
    };
    validate_origin_keys(
        context,
        &format!("metadata domain '{}'", site.name),
        &values_by_context.keys().cloned().collect(),
        provenance.context_origins.keys().cloned(),
        &context_origins_path,
    );
    validate_origin_keys(
        context,
        &format!("metadata domain '{}' context", site.name),
        &values_by_context.keys().cloned().collect(),
        provenance.value_origins.keys().cloned(),
        &value_origins_path,
    );
    for (context_name, origins) in &provenance.value_origins {
        if let Some(values) = values_by_context.get(context_name) {
            validate_origin_keys(
                context,
                &format!("metadata domain '{}' context '{}'", site.name, context_name),
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
}
