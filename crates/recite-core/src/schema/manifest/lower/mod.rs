mod content;
mod definitions;
mod domains;
mod functions;
mod types;
mod version;

use super::SchemaLoadReport;
use super::diagnostics::{MALFORMED_SHAPE, UNSUPPORTED_VERSION};
use super::raw::RawManifest;
use super::spans::{ManifestSpans, top_level_key_span};
use super::validate::{validate_domain_references, validate_type_references};
use crate::Diagnostic;
use crate::schema::ProjectSchema;

use content::{PendingReferences, lower_markup, lower_metadata};
use definitions::{lower_registries, lower_speakers, lower_types};
use domains::lower_metadata_domains;
use functions::{lower_conditions, lower_effects};
use version::{SchemaVersion, schema_version};

pub(crate) fn lower_manifest(file: String, source: &str, raw: RawManifest) -> SchemaLoadReport {
    let mut diagnostics = Vec::new();
    let mut schema = ProjectSchema::empty_v1();
    let mut spans = ManifestSpans::new();
    let mut pending_type_refs = Vec::new();
    let mut pending_domain_refs = Vec::new();

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
        &mut pending_type_refs,
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
