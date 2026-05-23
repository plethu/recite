use std::collections::BTreeSet;

use super::super::diagnostics::{DUPLICATE_DEFINITION, MALFORMED_SHAPE, diagnostic};
use super::super::raw::{Named, RawMarkupDefinition, RawMetadataDefinition};
use super::super::spans::ManifestSpans;
use super::super::validate::{
    PendingTypeReference, duplicate_definition, parse_metadata_target, validate_manifest_name,
};
use super::types::lower_type_reference;
use crate::Diagnostic;
use crate::schema::{MarkupDefinition, MetadataDefinition, ProjectSchema};

pub(super) fn lower_metadata(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawMetadataDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(diagnostics, "metadata name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "metadata", &entry.name, name_span);
            continue;
        }

        let mut targets = BTreeSet::new();
        for target in &entry.value.targets {
            let target_span = spans.next_value_span(file, source, target);
            let Some(metadata_target) = parse_metadata_target(target) else {
                diagnostics.push(diagnostic(
                    MALFORMED_SHAPE,
                    format!(
                        "metadata '{}' uses unsupported target '{}'",
                        entry.name, target
                    ),
                    target_span,
                ));
                continue;
            };

            if !targets.insert(metadata_target) {
                diagnostics.push(diagnostic(
                    DUPLICATE_DEFINITION,
                    format!("metadata '{}' repeats target '{}'", entry.name, target),
                    target_span,
                ));
            }
        }

        let (type_ref, type_ref_span, type_ref_is_valid) = lower_type_reference(
            file,
            source,
            spans,
            diagnostics,
            &entry.value.type_ref,
            format!(
                "metadata '{}' has invalid type reference '{}'",
                entry.name, entry.value.type_ref
            ),
        );
        if type_ref_is_valid {
            pending_type_refs.push(PendingTypeReference {
                owner: format!("metadata '{}'", entry.name),
                type_ref: type_ref.clone(),
                span: type_ref_span,
            });
        }

        schema.metadata.insert(
            entry.name,
            MetadataDefinition {
                targets,
                type_ref,
                repeatable: entry.value.repeatable,
            },
        );
    }
}

pub(super) fn lower_markup(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawMarkupDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = BTreeSet::new();
    for entry in entries {
        let name_span = spans.next_key_span(file, source, &entry.name);
        if !validate_manifest_name(diagnostics, "markup name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "markup", &entry.name, name_span);
            continue;
        }

        schema.markup.insert(
            entry.name,
            MarkupDefinition {
                requires_closing: entry.value.requires_closing,
                translatable: entry.value.translatable,
                allows_nesting: entry.value.allows_nesting.unwrap_or(true),
            },
        );
    }
}
