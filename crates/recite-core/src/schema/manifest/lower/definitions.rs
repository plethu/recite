use std::collections::BTreeSet;

use super::super::raw::{Named, RawRegistryDefinition, RawSpeakerDefinition, RawTypeDefinition};
use super::super::spans::ManifestSpans;
use super::super::validate::{
    duplicate_definition, validate_manifest_name, validate_non_empty_string,
};
use super::LoweringContext;
use super::producer::{
    ProvenanceLocation, lower_origin, lower_origin_value_map, lower_producer_fingerprints,
    validate_origin_keys,
};
use crate::schema::{
    EnumTypeDefinition, ProjectSchema, RegistryDefinition, SchemaTypeDefinition, SpeakerDefinition,
    schema_diagnostic,
};
use crate::{Diagnostic, DiagnosticArgumentValue};

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
        let entry_path = vec!["types".to_owned(), entry.name.clone()];
        let name_span = spans.key_span_at(file, source, &entry_path, &entry.name);
        if !validate_manifest_name(diagnostics, "type name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "type", &entry.name, name_span);
            continue;
        }

        let mut kind_path = entry_path.clone();
        kind_path.push("kind".to_owned());
        let kind_span = spans.value_span_at(file, source, &kind_path, &entry.value.kind);
        if entry.value.kind != "enum" {
            diagnostics.push(schema_diagnostic(
                super::super::diagnostics::MALFORMED_SHAPE,
                "diagnostic-schema-001-type-kind",
                format!(
                    "type '{}' uses unsupported kind '{}'",
                    entry.name, entry.value.kind
                ),
                kind_span,
                [
                    ("type", DiagnosticArgumentValue::String(entry.name.clone())),
                    (
                        "kind",
                        DiagnosticArgumentValue::String(entry.value.kind.clone()),
                    ),
                ],
            ));
            continue;
        }

        let values = canonical_string_values_at(
            &mut LoweringContext::new(file, source, spans, diagnostics),
            &format!("enum '{}'", entry.name),
            &entry.value.values,
            &entry_path,
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
    allow_duplicate_fingerprints: bool,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let entry_path = vec!["registries".to_owned(), entry.name.clone()];
        let name_span = spans.key_span_at(file, source, &entry_path, &entry.name);
        if !validate_manifest_name(diagnostics, "registry name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "registry", &entry.name, name_span);
            continue;
        }

        let values = canonical_string_values_at(
            &mut LoweringContext::new(file, source, spans, diagnostics),
            &format!("registry '{}'", entry.name),
            &entry.value.values,
            &entry_path,
        );
        let origin_path = {
            let mut path = entry_path.clone();
            path.push("origin".to_owned());
            path
        };
        let origin = lower_origin(
            &mut LoweringContext::new(file, source, spans, diagnostics),
            entry.value.origin,
            ProvenanceLocation {
                owner: &format!("registry '{}'", entry.name),
                span: name_span.clone(),
                path: &origin_path,
            },
        );
        let value_origins = lower_origin_value_map(
            &mut LoweringContext::new(file, source, spans, diagnostics),
            entry.value.value_origins,
            ProvenanceLocation {
                owner: &format!("registry '{}' value", entry.name),
                span: name_span.clone(),
                path: &entry_path,
            },
        );
        validate_origin_keys(
            &mut LoweringContext::new(file, source, spans, diagnostics),
            &format!("registry '{}'", entry.name),
            &values,
            value_origins.keys().cloned(),
            &{
                let mut path = entry_path.clone();
                path.push("value_origins".to_owned());
                path
            },
        );
        let producer_fingerprints = lower_producer_fingerprints(
            &mut LoweringContext::new(file, source, spans, diagnostics),
            entry.value.producer_fingerprints,
            &entry_path,
            &format!("registry '{}'", entry.name),
            allow_duplicate_fingerprints,
        );
        schema.registries.insert(
            entry.name,
            RegistryDefinition {
                values,
                origin,
                value_origins,
                producer_fingerprints,
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
        let entry_path = vec!["speakers".to_owned(), entry.name.clone()];
        let name_span = spans.key_span_at(file, source, &entry_path, &entry.name);
        if !validate_manifest_name(diagnostics, "speaker name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "speaker", &entry.name, name_span);
            continue;
        }

        if let Some(display_name) = &entry.value.display_name {
            let mut display_path = entry_path.clone();
            display_path.push("display_name".to_owned());
            let display_name_span = spans.value_span_at(file, source, &display_path, display_name);
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

pub(super) fn canonical_string_values_at(
    context: &mut LoweringContext<'_>,
    owner: &str,
    values: &[String],
    parent_path: &[String],
) -> BTreeSet<String> {
    let mut canonical = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let mut value_path = parent_path.to_vec();
        value_path.extend(["values".to_owned(), format!("[{index}]")]);
        let value_span = context.value_span_at(&value_path, value);
        if !validate_non_empty_string(
            context.diagnostics,
            "schema value",
            value,
            value_span.clone(),
        ) {
            continue;
        }
        if !canonical.insert(value.clone()) {
            context.diagnostics.push(schema_diagnostic(
                super::super::diagnostics::DUPLICATE_DEFINITION,
                "diagnostic-schema-003-value",
                format!("{owner} repeats value '{value}'"),
                value_span,
                [
                    ("owner", DiagnosticArgumentValue::String(owner.to_owned())),
                    ("value", DiagnosticArgumentValue::String(value.clone())),
                ],
            ));
        }
    }
    canonical
}
