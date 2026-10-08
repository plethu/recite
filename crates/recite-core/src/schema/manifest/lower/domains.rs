use std::collections::BTreeSet;

use super::super::diagnostics::MALFORMED_SHAPE;
use super::super::raw::{Named, RawMetadataDomainDefinition};
use super::super::validate::{
    PendingDomainReference, duplicate_definition, validate_manifest_name,
};
use super::LoweringContext;
use super::domains_provenance::{DomainKindFields, validate_domain_kind_fields};
use crate::schema::{MetadataDomainDefinition, ProjectSchema, schema_diagnostic};
use crate::{DiagnosticArgumentValue, SourceSpan};

mod contextual;
mod flat;

struct DomainSite {
    name: String,
    path: Vec<String>,
    span: SourceSpan,
}

pub(super) fn lower_metadata_domains(
    context: &mut LoweringContext<'_>,
    entries: Vec<Named<RawMetadataDomainDefinition>>,
    schema: &mut ProjectSchema,
    pending_domain_refs: &mut Vec<PendingDomainReference>,
    allow_duplicate_fingerprints: bool,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let path = vec!["metadata_domains".to_owned(), entry.name.clone()];
        let span = context.key_span_at(&path, &entry.name);
        if !validate_manifest_name(
            context.diagnostics,
            "metadata domain name",
            &entry.name,
            span.clone(),
        ) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(context.diagnostics, "metadata domain", &entry.name, span);
            continue;
        }
        let site = DomainSite {
            name: entry.name,
            path,
            span,
        };
        if let Some(definition) = lower_domain(
            context,
            &site,
            entry.value,
            pending_domain_refs,
            allow_duplicate_fingerprints,
        ) {
            schema.metadata_domains.insert(site.name, definition);
        }
    }
}

fn lower_domain(
    context: &mut LoweringContext<'_>,
    site: &DomainSite,
    raw: RawMetadataDomainDefinition,
    pending_domain_refs: &mut Vec<PendingDomainReference>,
    allow_duplicate_fingerprints: bool,
) -> Option<MetadataDomainDefinition> {
    if !validate_domain_kind_fields(
        context.diagnostics,
        DomainKindFields {
            kind: &raw.kind,
            has_values: raw.values.is_some(),
            has_selector: raw.selector.is_some(),
            has_values_by_context: raw.values_by_context.is_some(),
            has_missing_context: raw.missing_context.is_some(),
            has_context_origins: raw.context_origins.is_some(),
            owner: &site.name,
            span: site.span.clone(),
        },
    ) {
        return None;
    }
    match raw.kind.as_str() {
        "flat" => flat::lower(context, site, raw, allow_duplicate_fingerprints)
            .map(MetadataDomainDefinition::Flat),
        "contextual" => contextual::lower(
            context,
            site,
            raw,
            pending_domain_refs,
            allow_duplicate_fingerprints,
        )
        .map(MetadataDomainDefinition::Contextual),
        other => {
            let mut kind_path = site.path.clone();
            kind_path.push("kind".to_owned());
            let kind_span = context.value_span_at(&kind_path, other);
            context.diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-domain-kind",
                format!(
                    "metadata domain '{}' uses unsupported kind '{}'",
                    site.name, other
                ),
                kind_span,
                [
                    ("domain", DiagnosticArgumentValue::String(site.name.clone())),
                    ("kind", DiagnosticArgumentValue::String(other.to_owned())),
                ],
            ));
            None
        }
    }
}
