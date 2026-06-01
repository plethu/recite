use std::collections::BTreeSet;

use super::super::diagnostics::{DUPLICATE_DEFINITION, MALFORMED_SHAPE};
use super::super::raw::{Named, RawMarkupDefinition, RawMetadataDefinition};
use super::super::spans::ManifestSpans;
use super::super::validate::{
    PendingDomainReference, PendingTypeReference, duplicate_definition, parse_metadata_target,
    validate_manifest_name,
};
use super::types::lower_type_reference;
use crate::Diagnostic;
use crate::schema::{MarkupDefinition, MetadataDefinition, ProjectSchema};

pub(super) struct PendingReferences<'a> {
    pub(super) type_refs: &'a mut Vec<PendingTypeReference>,
    pub(super) domain_refs: &'a mut Vec<PendingDomainReference>,
}

pub(super) fn lower_metadata(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    entries: Vec<Named<RawMetadataDefinition>>,
    schema: &mut ProjectSchema,
    diagnostics: &mut Vec<Diagnostic>,
    pending_refs: PendingReferences<'_>,
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
                diagnostics.push(Diagnostic::error(
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
                diagnostics.push(Diagnostic::error(
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
            pending_refs.type_refs.push(PendingTypeReference {
                owner: format!("metadata '{}'", entry.name),
                type_ref: type_ref.clone(),
                span: type_ref_span,
            });
        }

        if let Some(domain) = &entry.value.domain {
            let domain_span = spans.next_value_span(file, source, domain);
            if type_ref != crate::schema::SchemaTypeRef::Symbol {
                diagnostics.push(Diagnostic::error(
                    MALFORMED_SHAPE,
                    format!(
                        "metadata '{}' uses a metadata domain but has non-symbol type '{}'",
                        entry.name, entry.value.type_ref
                    ),
                    domain_span,
                ));
            } else {
                pending_refs.domain_refs.push(PendingDomainReference {
                    owner: format!("metadata '{}'", entry.name),
                    domain: domain.clone(),
                    require_flat: false,
                    span: domain_span,
                });
            }
        }

        schema.metadata.insert(
            entry.name,
            MetadataDefinition {
                targets,
                type_ref,
                repeatable: entry.value.repeatable,
                domain: entry.value.domain,
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
