use std::collections::BTreeSet;

use super::super::raw::{Named, RawMarkupDefinition, RawMetadataDefinition};
use super::super::spans::ManifestSpans;
use super::super::validate::{
    PendingDomainReference, PendingTypeReference, duplicate_definition, parse_metadata_target,
    validate_manifest_name,
};
use super::types::{TypeReferenceContext, lower_type_reference_at_with_context};
use crate::schema::{MarkupDefinition, MetadataDefinition, ProjectSchema, schema_diagnostic};
use crate::{Diagnostic, DiagnosticArgumentValue};

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
        let entry_path = vec!["metadata".to_owned(), entry.name.clone()];
        let name_span = spans.key_span_at(file, source, &entry_path, &entry.name);
        if !validate_manifest_name(diagnostics, "metadata name", &entry.name, name_span.clone()) {
            continue;
        }
        if !seen.insert(entry.name.clone()) {
            duplicate_definition(diagnostics, "metadata", &entry.name, name_span);
            continue;
        }

        let mut targets = BTreeSet::new();
        for (index, target) in entry.value.targets.iter().enumerate() {
            let mut target_path = entry_path.clone();
            target_path.extend(["targets".to_owned(), format!("[{index}]")]);
            let target_span = spans.value_span_at(file, source, &target_path, target);
            let Some(metadata_target) = parse_metadata_target(target) else {
                diagnostics.push(schema_diagnostic(
                    super::super::diagnostics::MALFORMED_SHAPE,
                    "diagnostic-schema-001-metadata-target",
                    format!(
                        "metadata '{}' uses unsupported target '{}'",
                        entry.name, target
                    ),
                    target_span,
                    [
                        (
                            "metadata",
                            DiagnosticArgumentValue::String(entry.name.clone()),
                        ),
                        ("target", DiagnosticArgumentValue::String(target.clone())),
                    ],
                ));
                continue;
            };

            if !targets.insert(metadata_target) {
                diagnostics.push(schema_diagnostic(
                    super::super::diagnostics::DUPLICATE_DEFINITION,
                    "diagnostic-schema-003-metadata-target",
                    format!("metadata '{}' repeats target '{}'", entry.name, target),
                    target_span,
                    [
                        (
                            "metadata",
                            DiagnosticArgumentValue::String(entry.name.clone()),
                        ),
                        ("target", DiagnosticArgumentValue::String(target.clone())),
                    ],
                ));
            }
        }

        let mut type_path = entry_path.clone();
        type_path.push("type".to_owned());
        let (type_ref, type_ref_span, type_ref_is_valid) = lower_type_reference_at_with_context(
            file,
            source,
            spans,
            diagnostics,
            &entry.value.type_ref,
            &type_path,
            format!(
                "metadata '{}' has invalid type reference '{}'",
                entry.name, entry.value.type_ref
            ),
            TypeReferenceContext::Metadata {
                metadata: entry.name.clone(),
            },
        );
        if type_ref_is_valid {
            pending_refs.type_refs.push(PendingTypeReference {
                owner: format!("metadata '{}'", entry.name),
                type_ref: type_ref.clone(),
                span: type_ref_span.clone(),
            });
        }
        if matches!(type_ref, crate::schema::SchemaTypeRef::Array(_)) {
            diagnostics.push(schema_diagnostic(
                super::super::diagnostics::MALFORMED_SHAPE,
                "diagnostic-schema-001-metadata-array-type",
                format!(
                    "metadata '{}' uses projection-only array type '{}'",
                    entry.name, entry.value.type_ref
                ),
                type_ref_span,
                [
                    (
                        "metadata",
                        DiagnosticArgumentValue::String(entry.name.clone()),
                    ),
                    (
                        "type_ref",
                        DiagnosticArgumentValue::String(entry.value.type_ref.clone()),
                    ),
                ],
            ));
        }

        if let Some(domain) = &entry.value.domain {
            let mut domain_path = entry_path.clone();
            domain_path.push("domain".to_owned());
            let domain_span = spans.value_span_at(file, source, &domain_path, domain);
            if type_ref != crate::schema::SchemaTypeRef::Symbol {
                diagnostics.push(schema_diagnostic(
                    super::super::diagnostics::MALFORMED_SHAPE,
                    "diagnostic-schema-001-metadata-domain-type",
                    format!(
                        "metadata '{}' uses a metadata domain but has non-symbol type '{}'",
                        entry.name, entry.value.type_ref
                    ),
                    domain_span,
                    [
                        (
                            "metadata",
                            DiagnosticArgumentValue::String(entry.name.clone()),
                        ),
                        (
                            "type_ref",
                            DiagnosticArgumentValue::String(entry.value.type_ref.clone()),
                        ),
                    ],
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
        let entry_path = vec!["markup".to_owned(), entry.name.clone()];
        let name_span = spans.key_span_at(file, source, &entry_path, &entry.name);
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
