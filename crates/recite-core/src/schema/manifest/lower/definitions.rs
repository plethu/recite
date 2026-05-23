use std::collections::BTreeSet;

use super::super::diagnostics::{DUPLICATE_DEFINITION, MALFORMED_SHAPE, diagnostic};
use super::super::raw::{Named, RawRegistryDefinition, RawSpeakerDefinition, RawTypeDefinition};
use super::super::spans::ManifestSpans;
use super::super::validate::{
    duplicate_definition, validate_manifest_name, validate_non_empty_string,
};
use crate::Diagnostic;
use crate::schema::{
    EnumTypeDefinition, ProjectSchema, RegistryDefinition, SchemaTypeDefinition, SpeakerDefinition,
};

pub(super) fn lower_types(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawTypeDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(diagnostics, "type name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "type", &entry.name, name_span);
            continue;
        }

        let kind_span = spans.next_value_span(file, source, &entry.value.kind);
        if entry.value.kind != "enum" {
            diagnostics.push(diagnostic(
                MALFORMED_SHAPE,
                format!(
                    "type '{}' uses unsupported kind '{}'",
                    entry.name, entry.value.kind
                ),
                kind_span,
            ));
            continue;
        }

        let values = canonical_string_values(
            file,
            source,
            spans,
            diagnostics,
            &format!("enum '{}'", entry.name),
            &entry.value.values,
        );
        schema.types.insert(
            entry.name,
            SchemaTypeDefinition::Enum(EnumTypeDefinition { values }),
        );
    }
}

pub(super) fn lower_registries(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawRegistryDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(diagnostics, "registry name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "registry", &entry.name, name_span);
            continue;
        }

        let values = canonical_string_values(
            file,
            source,
            spans,
            diagnostics,
            &format!("registry '{}'", entry.name),
            &entry.value.values,
        );
        if let Some(origin) = &entry.value.origin {
            let origin_span = spans.next_value_span(file, source, origin);
            validate_non_empty_string(diagnostics, "registry origin", origin, origin_span);
        }
        schema.registries.insert(
            entry.name,
            RegistryDefinition {
                values,
                origin: entry.value.origin,
            },
        );
    }
}

pub(super) fn lower_speakers(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawSpeakerDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(diagnostics, "speaker name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "speaker", &entry.name, name_span);
            continue;
        }

        if let Some(display_name) = &entry.value.display_name {
            let display_name_span = spans.next_value_span(file, source, display_name);
            validate_non_empty_string(
                diagnostics,
                "speaker display_name",
                display_name,
                display_name_span,
            );
        }
        schema.speakers.insert(
            entry.name,
            SpeakerDefinition {
                display_name: entry.value.display_name,
            },
        );
    }
}

pub(super) fn canonical_string_values(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    owner: &str,
    values: &[String],
) -> BTreeSet<String> {
    let mut canonical = BTreeSet::new();
    for value in values {
        let value_span = spans.next_value_span(file, source, value);
        if !validate_non_empty_string(diagnostics, "schema value", value, value_span.clone()) {
            continue;
        }
        if !canonical.insert(value.clone()) {
            diagnostics.push(diagnostic(
                DUPLICATE_DEFINITION,
                format!("{owner} repeats value '{value}'"),
                value_span,
            ));
        }
    }
    canonical
}
