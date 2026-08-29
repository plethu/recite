mod availability;
mod availability_bindings;
mod content;
mod definitions;
mod domains;
mod domains_context;
mod domains_provenance;
mod functions;
mod numeric;
mod parameters;
mod producer;
mod producer_provenance;
mod projection;
mod types;
mod version;

use super::SchemaLoadReport;
use super::diagnostics::{MALFORMED_SHAPE, UNSUPPORTED_VERSION};
use super::raw::RawManifest;
use super::spans::{ManifestSpans, TomlSpanIndex};
use super::validate::{validate_domain_references, validate_type_references};
use crate::DiagnosticArgumentValue;
use crate::schema::{ProjectSchema, schema_diagnostic};

use availability::{lower_availability_reasons, validate_condition_availability_reason_mappings};
use content::{PendingReferences, lower_markup, lower_metadata};
use definitions::{lower_registries, lower_speakers, lower_types};
use domains::lower_metadata_domains;
use functions::{FunctionPendingReferences, lower_conditions, lower_effects};
use projection::{lower_presentation_projectors, lower_projection_queries};
use version::{SchemaVersion, schema_version, toml_schema_version};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ManifestSourceFormat {
    Json,
    Toml,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ManifestLoadOptions {
    pub(crate) allow_duplicate_producer_fingerprints: bool,
}

pub(crate) fn lower_manifest(
    file: String,
    source: &str,
    raw: RawManifest,
    options: ManifestLoadOptions,
) -> SchemaLoadReport {
    lower_manifest_with_format(file, source, raw, options, ManifestSourceFormat::Json, None)
}

pub(crate) fn lower_manifest_with_format(
    file: String,
    source: &str,
    raw: RawManifest,
    options: ManifestLoadOptions,
    format: ManifestSourceFormat,
    toml_spans: Option<&TomlSpanIndex>,
) -> SchemaLoadReport {
    let mut diagnostics = Vec::new();
    let mut schema = ProjectSchema::empty_v1();
    let mut spans = ManifestSpans::new_with_format(format, toml_spans);
    let mut pending_type_refs = Vec::new();
    let mut pending_domain_refs = Vec::new();
    let mut pending_availability_reason_mappings = Vec::new();

    schema.producer_metadata = producer::lower_producer_metadata(
        &mut spans,
        &file,
        source,
        raw.producer,
        raw.content_fingerprint,
        raw.schema_export_version,
        raw.inclusion_policy,
        raw.producer_fingerprints,
        options.allow_duplicate_producer_fingerprints,
        &mut diagnostics,
    );

    let source_version = match format {
        ManifestSourceFormat::Json => schema_version(source, &raw.schema_version),
        ManifestSourceFormat::Toml => toml_schema_version(source, &raw.schema_version, toml_spans),
    };
    match source_version {
        SchemaVersion::One => {}
        SchemaVersion::Unsupported(version) => diagnostics.push(schema_diagnostic(
            UNSUPPORTED_VERSION,
            "diagnostic-schema-002-unsupported-version",
            format!("unsupported schema manifest version {version}"),
            spans.root_key_span(&file, source, "schema_version"),
            [(
                "version",
                DiagnosticArgumentValue::String(version.to_owned()),
            )],
        )),
        SchemaVersion::Malformed => diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-schema-version-type",
            "schema_version must be an integer",
            spans.root_key_span(&file, source, "schema_version"),
            std::iter::empty::<(String, DiagnosticArgumentValue)>(),
        )),
    }

    spans.enter_section(source, "types");
    lower_types(
        &file,
        source,
        &mut spans,
        raw.types,
        &mut schema,
        &mut diagnostics,
    );
    spans.enter_section(source, "registries");
    lower_registries(
        &file,
        source,
        &mut spans,
        raw.registries,
        &mut schema,
        &mut diagnostics,
        options.allow_duplicate_producer_fingerprints,
    );
    spans.enter_section(source, "speakers");
    lower_speakers(
        &file,
        source,
        &mut spans,
        raw.speakers,
        &mut schema,
        &mut diagnostics,
    );
    spans.enter_section(source, "conditions");
    lower_conditions(
        &file,
        source,
        &mut spans,
        raw.conditions,
        &mut schema,
        &mut diagnostics,
        FunctionPendingReferences {
            type_refs: &mut pending_type_refs,
            availability_reason_mappings: &mut pending_availability_reason_mappings,
        },
    );
    spans.enter_section(source, "availability_reasons");
    lower_availability_reasons(
        &file,
        source,
        &mut spans,
        raw.availability_reasons,
        &mut schema,
        &mut diagnostics,
        &mut pending_type_refs,
    );
    spans.enter_section(source, "conditions");
    validate_condition_availability_reason_mappings(
        &file,
        source,
        &mut spans,
        pending_availability_reason_mappings,
        &mut schema,
        &mut diagnostics,
        format,
    );
    spans.enter_section(source, "effects");
    lower_effects(
        &file,
        source,
        &mut spans,
        raw.effects,
        &mut schema,
        &mut diagnostics,
        &mut pending_type_refs,
    );
    spans.enter_section(source, "metadata_domains");
    lower_metadata_domains(
        &file,
        source,
        &mut spans,
        raw.metadata_domains,
        &mut schema,
        &mut diagnostics,
        &mut pending_domain_refs,
        options.allow_duplicate_producer_fingerprints,
    );
    spans.enter_section(source, "metadata");
    lower_metadata(
        &file,
        source,
        &mut spans,
        raw.metadata,
        &mut schema,
        &mut diagnostics,
        PendingReferences {
            type_refs: &mut pending_type_refs,
            domain_refs: &mut pending_domain_refs,
        },
    );
    spans.enter_section(source, "projection_queries");
    lower_projection_queries(
        &file,
        source,
        &mut spans,
        raw.projection_queries,
        &mut schema,
        &mut diagnostics,
        &mut pending_type_refs,
    );
    spans.enter_section(source, "presentation_projectors");
    lower_presentation_projectors(
        &file,
        source,
        &mut spans,
        raw.presentation_projectors,
        &mut schema,
        &mut diagnostics,
        &mut pending_type_refs,
    );
    spans.enter_section(source, "markup");
    lower_markup(
        &file,
        source,
        &mut spans,
        raw.markup,
        &mut schema,
        &mut diagnostics,
    );
    validate_type_references(&schema, &pending_type_refs, &mut diagnostics);
    validate_domain_references(&schema, &pending_domain_refs, &mut diagnostics);

    let schema = diagnostics.is_empty().then_some(schema);
    SchemaLoadReport {
        schema,
        diagnostics,
    }
}
