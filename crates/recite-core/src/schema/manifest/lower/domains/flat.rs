use super::super::super::diagnostics::MALFORMED_SHAPE;
use super::super::super::raw::RawMetadataDomainDefinition;
use super::super::definitions::canonical_string_values_at;
use super::super::domains_provenance::{FlatDomainProvenanceInput, lower_flat_domain_provenance};
use super::super::producer::{ProvenanceLocation, validate_origin_keys};
use super::{DomainSite, LoweringContext};
use crate::DiagnosticArgumentValue;
use crate::schema::{FlatMetadataDomain, schema_diagnostic};

pub(super) fn lower(
    context: &mut LoweringContext<'_>,
    site: &DomainSite,
    raw: RawMetadataDomainDefinition,
    allow_duplicate_fingerprints: bool,
) -> Option<FlatMetadataDomain> {
    let Some(values) = raw.values.as_ref() else {
        context.diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-domain-values",
            format!("metadata domain '{}' requires values", site.name),
            site.span.clone(),
            [("domain", DiagnosticArgumentValue::String(site.name.clone()))],
        ));
        return None;
    };
    let values = canonical_string_values_at(
        context,
        &format!("metadata domain '{}'", site.name),
        values,
        &site.path,
    );
    let provenance = lower_flat_domain_provenance(
        context,
        FlatDomainProvenanceInput {
            origin: raw.origin,
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
    let value_origins_path = {
        let mut path = site.path.clone();
        path.push("value_origins".to_owned());
        path
    };
    validate_origin_keys(
        context,
        &format!("metadata domain '{}'", site.name),
        &values,
        provenance.value_origins.keys().cloned(),
        &value_origins_path,
    );
    Some(FlatMetadataDomain { values, provenance })
}
