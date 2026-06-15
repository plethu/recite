mod availability;
mod content;
mod definitions;
mod domains;
mod functions;
mod projection;
mod types;
mod version;

use super::SchemaLoadReport;
use super::diagnostics::{MALFORMED_SHAPE, UNSUPPORTED_VERSION};
use super::raw::RawManifest;
use super::spans::{ManifestSpans, top_level_key_span};
use super::validate::{validate_domain_references, validate_type_references};
use crate::Diagnostic;
use crate::schema::ProjectSchema;

use availability::{lower_availability_reasons, validate_condition_availability_reason_mappings};
use content::{PendingReferences, lower_markup, lower_metadata};
use definitions::{lower_registries, lower_speakers, lower_types};
use domains::lower_metadata_domains;
use functions::{FunctionPendingReferences, lower_conditions, lower_effects};
use projection::{lower_presentation_projectors, lower_projection_queries};
use version::{SchemaVersion, schema_version};

pub(crate) fn lower_manifest(file: String, source: &str, raw: RawManifest) -> SchemaLoadReport {
    let mut diagnostics = Vec::new();
    let mut schema = ProjectSchema::empty_v1();
    let mut spans = ManifestSpans::new();
    let mut pending_type_refs = Vec::new();
    let mut pending_domain_refs = Vec::new();
    let mut pending_availability_reason_mappings = Vec::new();

    match schema_version(source, &raw.schema_version) {
        SchemaVersion::One => {}
        SchemaVersion::Unsupported(version) => diagnostics.push(Diagnostic::error(
            UNSUPPORTED_VERSION,
            format!("unsupported schema manifest version {version}"),
            top_level_key_span(&file, source, "schema_version"),
        )),
        SchemaVersion::Malformed => diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            "schema_version must be an integer",
            top_level_key_span(&file, source, "schema_version"),
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
