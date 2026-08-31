use recite_compiler::{SchemaCapability, SchemaDeclarationProvenance, SchemaSummary};

use crate::error::CliError;

use super::capabilities::capability_json;
use super::definitions::*;
use super::fingerprints::fingerprints_json;
use super::freshness::freshness_json;
use super::model::{DeclarationProjection, SchemaInspectionProjection, SourceProjection};
use super::path::MachinePathProjection;
use super::provenance::{identity_json, ownership_json, provenance_json};
use super::{INSPECTION_FORMAT_VERSION, input::InputFormat};

pub(super) fn from_source(
    source: &recite_core::SchemaSource,
    path: MachinePathProjection,
) -> Result<SchemaInspectionProjection, CliError> {
    let summary = SchemaSummary::from_source(source);
    from_summary(&summary, source.schema(), InputFormat::StandaloneToml, path)
}

pub(super) fn from_generated(
    schema: &recite_core::ProjectSchema,
    path: MachinePathProjection,
) -> Result<SchemaInspectionProjection, CliError> {
    let summary = SchemaSummary::from_schema(schema);
    from_summary(&summary, schema, InputFormat::GeneratedJson, path)
}

fn from_summary(
    summary: &SchemaSummary,
    schema: &recite_core::ProjectSchema,
    format: InputFormat,
    path: MachinePathProjection,
) -> Result<SchemaInspectionProjection, CliError> {
    let ownership = ownership_json(summary.ownership());
    let producer = summary.ownership().producer().map(identity_json);
    Ok(SchemaInspectionProjection {
        format_version: INSPECTION_FORMAT_VERSION,
        schema_version: summary.schema_version(),
        source: SourceProjection {
            format: format.name(),
            path,
            read_only: matches!(format, InputFormat::GeneratedJson),
        },
        ownership,
        capability: capability_json(summary.capability()),
        producer,
        fingerprints: fingerprints_json(summary, schema)?,
        freshness: freshness_json(summary.freshness()),
        types: schema
            .types
            .iter()
            .zip(summary.types())
            .map(|((name, definition), item)| {
                declaration(
                    "type",
                    name,
                    json_type_definition(definition),
                    item.provenance(),
                    item.capability(),
                )
            })
            .collect(),
        registries: schema
            .registries
            .iter()
            .zip(summary.registries())
            .map(|((name, definition), item)| {
                declaration(
                    "registry",
                    name,
                    json_registry_definition(definition),
                    item.provenance(),
                    item.capability(),
                )
            })
            .collect(),
        speakers: schema
            .speakers
            .iter()
            .zip(summary.speakers())
            .map(|((name, definition), item)| {
                declaration(
                    "speaker",
                    name,
                    serde_json::json!({ "display_name": definition.display_name }),
                    item.provenance(),
                    item.capability(),
                )
            })
            .collect(),
        conditions: schema
            .conditions
            .iter()
            .zip(summary.conditions())
            .map(|((name, definition), item)| {
                declaration(
                    "condition",
                    name,
                    json_condition_definition(definition),
                    item.provenance(),
                    item.capability(),
                )
            })
            .collect(),
        availability_reasons: schema
            .availability_reasons
            .iter()
            .zip(summary.availability_reasons())
            .map(|((name, definition), item)| {
                declaration(
                    "availability_reason",
                    name.as_str(),
                    json_availability_reason_definition(definition),
                    item.provenance(),
                    item.capability(),
                )
            })
            .collect(),
        effects: schema
            .effects
            .iter()
            .zip(summary.effects())
            .map(|((name, definition), item)| {
                declaration(
                    "effect",
                    name,
                    json_effect_definition(definition),
                    item.provenance(),
                    item.capability(),
                )
            })
            .collect(),
        metadata_domains: schema
            .metadata_domains
            .iter()
            .zip(summary.metadata_domains())
            .map(|((name, definition), item)| {
                declaration(
                    "metadata_domain",
                    name,
                    json_metadata_domain_definition(definition),
                    item.provenance(),
                    item.capability(),
                )
            })
            .collect(),
        metadata: schema
            .metadata
            .iter()
            .zip(summary.metadata())
            .map(|((name, definition), item)| {
                declaration(
                    "metadata",
                    name,
                    json_metadata_definition(definition),
                    item.provenance(),
                    item.capability(),
                )
            })
            .collect(),
        projection_queries: schema
            .projection_queries
            .iter()
            .zip(summary.projection_queries())
            .map(|((name, definition), item)| {
                declaration(
                    "projection_query",
                    name,
                    json_projection_query_function(definition),
                    item.provenance(),
                    item.capability(),
                )
            })
            .collect(),
        presentation_projectors: schema
            .presentation_projectors
            .iter()
            .zip(summary.presentation_projectors())
            .map(|((name, definition), item)| {
                declaration(
                    "presentation_projector",
                    name,
                    json_presentation_projector(definition),
                    item.provenance(),
                    item.capability(),
                )
            })
            .collect(),
        markup: schema
            .markup
            .iter()
            .zip(summary.markup())
            .map(|((name, definition), item)| {
                declaration(
                    "markup",
                    name,
                    serde_json::json!({
                        "requires_closing": definition.requires_closing,
                        "translatable": definition.translatable,
                        "allows_nesting": definition.allows_nesting,
                    }),
                    item.provenance(),
                    item.capability(),
                )
            })
            .collect(),
    })
}

fn declaration(
    kind: &'static str,
    name: &str,
    definition: serde_json::Value,
    provenance: &SchemaDeclarationProvenance,
    capability: &SchemaCapability,
) -> DeclarationProjection {
    DeclarationProjection {
        kind,
        name: name.to_owned(),
        definition,
        provenance: provenance_json(provenance),
        capability: capability_json(capability),
    }
}

#[cfg(test)]
mod tests;
