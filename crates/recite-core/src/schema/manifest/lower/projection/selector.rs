use std::collections::BTreeSet;

use super::super::super::diagnostics::{
    DUPLICATE_DEFINITION, INVALID_TYPE_REFERENCE, MALFORMED_SHAPE,
};
use super::super::super::raw::RawProjectionSelector;
use super::super::super::spans::ManifestSpans;
use super::super::super::validate::{
    parse_metadata_target, validate_manifest_name, validate_non_empty_string,
};
use crate::schema::{MetadataDefinition, MetadataTarget, ProjectSchema, SchemaProjectionSelector};
use crate::{AvailabilityReasonId, Diagnostic};

pub(super) fn lower_selector(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    raw: RawProjectionSelector,
) -> Option<SchemaProjectionSelector> {
    match raw {
        RawProjectionSelector::RuntimeEvent { event } => {
            let span = spans.next_value_span(file, source, &event);
            validate_non_empty_string(diagnostics, "projection runtime event kind", &event, span);
            Some(SchemaProjectionSelector::RuntimeEvent { kind: event })
        }
        RawProjectionSelector::MetadataKey { target, key } => {
            let target =
                lower_projector_metadata_target(file, source, spans, diagnostics, &target)?;
            validate_metadata_key_target(
                diagnostics,
                schema,
                projector,
                &key,
                target,
                spans.next_value_span(file, source, &key),
            );
            Some(SchemaProjectionSelector::MetadataKey { target, key })
        }
        RawProjectionSelector::MetadataSet {
            target,
            required_keys,
        } => {
            let target =
                lower_projector_metadata_target(file, source, spans, diagnostics, &target)?;
            let mut seen_keys = BTreeSet::new();
            for key in &required_keys {
                let key_span = spans.next_value_span(file, source, key);
                validate_metadata_key_target(
                    diagnostics,
                    schema,
                    projector,
                    key,
                    target,
                    key_span.clone(),
                );
                if !seen_keys.insert(key.clone()) {
                    diagnostics.push(Diagnostic::error(
                        DUPLICATE_DEFINITION,
                        format!("projector '{projector}' repeats required metadata key '{key}'"),
                        key_span,
                    ));
                }
            }
            Some(SchemaProjectionSelector::MetadataSet {
                target,
                required_keys,
            })
        }
        RawProjectionSelector::AvailabilityReason { reason } => {
            let reason_span = spans.next_value_span(file, source, &reason);
            if !validate_manifest_name(
                diagnostics,
                "availability reason id",
                &reason,
                reason_span.clone(),
            ) {
                return None;
            }
            let Ok(reason_id) = AvailabilityReasonId::new(reason.clone()) else {
                return None;
            };
            if !schema.availability_reasons.contains_key(&reason_id) {
                diagnostics.push(Diagnostic::error(
                    INVALID_TYPE_REFERENCE,
                    format!(
                        "projector '{projector}' references unknown availability reason '{reason}'"
                    ),
                    reason_span,
                ));
            }
            Some(SchemaProjectionSelector::AvailabilityReason { reason_id })
        }
    }
}

fn lower_projector_metadata_target(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    raw: &str,
) -> Option<MetadataTarget> {
    let span = spans.next_value_span(file, source, raw);
    parse_metadata_target(raw).or_else(|| {
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!("presentation projector uses unsupported metadata target '{raw}'"),
            span,
        ));
        None
    })
}

pub(super) fn validate_metadata_key_target(
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    key: &str,
    target: MetadataTarget,
    span: crate::SourceSpan,
) -> Option<MetadataDefinition> {
    let Some(metadata) = schema.metadata.get(key) else {
        diagnostics.push(Diagnostic::error(
            INVALID_TYPE_REFERENCE,
            format!("projector '{projector}' references unknown metadata key '{key}'"),
            span,
        ));
        return None;
    };
    if !metadata.targets.contains(&target) {
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!(
                "projector '{projector}' references metadata key '{key}' on unsupported target '{}'",
                metadata_target_name(target)
            ),
            span,
        ));
    }
    Some(metadata.clone())
}

pub(super) fn selector_metadata_target(
    selector: &SchemaProjectionSelector,
) -> Option<MetadataTarget> {
    match selector {
        SchemaProjectionSelector::MetadataKey { target, .. }
        | SchemaProjectionSelector::MetadataSet { target, .. } => Some(*target),
        _ => None,
    }
}

fn metadata_target_name(target: MetadataTarget) -> &'static str {
    match target {
        MetadataTarget::Block => "block",
        MetadataTarget::Choice => "choice",
        MetadataTarget::Line => "line",
        MetadataTarget::Project => "project",
    }
}
