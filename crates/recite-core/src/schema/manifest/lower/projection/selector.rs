use std::collections::BTreeSet;

use super::super::super::diagnostics::{
    DUPLICATE_DEFINITION, INVALID_TYPE_REFERENCE, MALFORMED_SHAPE,
};
use super::super::super::raw::RawProjectionSelector;
use super::super::super::validate::{
    parse_metadata_target, validate_manifest_name, validate_non_empty_string,
};
use super::super::LoweringContext;
use crate::schema::schema_diagnostic;
use crate::schema::{MetadataDefinition, MetadataTarget, ProjectSchema, SchemaProjectionSelector};
use crate::{AvailabilityReasonId, Diagnostic, DiagnosticArgumentValue};

pub(super) fn lower_selector(
    lowering: &mut LoweringContext<'_>,
    schema: &ProjectSchema,
    projector: &str,
    raw: RawProjectionSelector,
    projector_path: &[String],
) -> Option<SchemaProjectionSelector> {
    match raw {
        RawProjectionSelector::RuntimeEvent { event } => {
            let mut path = projector_path.to_vec();
            path.extend(["candidates".to_owned(), "event".to_owned()]);
            let span = lowering.value_span_at(&path, &event);
            validate_non_empty_string(
                lowering.diagnostics,
                "projection runtime event kind",
                &event,
                span,
            );
            Some(SchemaProjectionSelector::RuntimeEvent { kind: event })
        }
        RawProjectionSelector::MetadataKey { target, key } => {
            let mut target_path = projector_path.to_vec();
            target_path.extend(["candidates".to_owned(), "target".to_owned()]);
            let target = lower_projector_metadata_target(lowering, &target, &target_path)?;
            let mut key_path = projector_path.to_vec();
            key_path.extend(["candidates".to_owned(), "key".to_owned()]);
            let key_span = lowering.value_span_at(&key_path, &key);
            validate_metadata_key_target(
                lowering.diagnostics,
                schema,
                projector,
                &key,
                target,
                key_span,
            );
            Some(SchemaProjectionSelector::MetadataKey { target, key })
        }
        RawProjectionSelector::MetadataSet {
            target,
            required_keys,
        } => {
            let mut target_path = projector_path.to_vec();
            target_path.extend(["candidates".to_owned(), "target".to_owned()]);
            let target = lower_projector_metadata_target(lowering, &target, &target_path)?;
            let mut seen_keys = BTreeSet::new();
            for (index, key) in required_keys.iter().enumerate() {
                let mut key_path = projector_path.to_vec();
                key_path.extend([
                    "candidates".to_owned(),
                    "required_keys".to_owned(),
                    format!("[{index}]"),
                ]);
                let key_span = lowering.value_span_at(&key_path, key);
                validate_metadata_key_target(
                    lowering.diagnostics,
                    schema,
                    projector,
                    key,
                    target,
                    key_span.clone(),
                );
                if !seen_keys.insert(key.clone()) {
                    lowering.diagnostics.push(schema_diagnostic(
                        DUPLICATE_DEFINITION,
                        "diagnostic-schema-003-required-metadata",
                        format!("projector '{projector}' repeats required metadata key '{key}'"),
                        key_span,
                        [
                            (
                                "projector",
                                DiagnosticArgumentValue::String(projector.to_owned()),
                            ),
                            ("key", DiagnosticArgumentValue::String(key.clone())),
                        ],
                    ));
                }
            }
            Some(SchemaProjectionSelector::MetadataSet {
                target,
                required_keys,
            })
        }
        RawProjectionSelector::AvailabilityReason { reason } => {
            let mut reason_path = projector_path.to_vec();
            reason_path.extend(["candidates".to_owned(), "reason".to_owned()]);
            let reason_span = lowering.value_span_at(&reason_path, &reason);
            if !validate_manifest_name(
                lowering.diagnostics,
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
                lowering.diagnostics.push(schema_diagnostic(
                    INVALID_TYPE_REFERENCE,
                    "diagnostic-schema-004-unknown-projection-reason",
                    format!(
                        "projector '{projector}' references unknown availability reason '{reason}'"
                    ),
                    reason_span,
                    [
                        (
                            "projector",
                            DiagnosticArgumentValue::String(projector.to_owned()),
                        ),
                        ("reason", DiagnosticArgumentValue::String(reason.clone())),
                    ],
                ));
            }
            Some(SchemaProjectionSelector::AvailabilityReason { reason_id })
        }
    }
}

fn lower_projector_metadata_target(
    lowering: &mut LoweringContext<'_>,
    raw: &str,
    path: &[String],
) -> Option<MetadataTarget> {
    let span = lowering.value_span_at(path, raw);
    parse_metadata_target(raw).or_else(|| {
        lowering.diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-projection-selector-target",
            format!("presentation projector uses unsupported metadata target '{raw}'"),
            span,
            [("target", DiagnosticArgumentValue::String(raw.to_owned()))],
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
        diagnostics.push(schema_diagnostic(
            INVALID_TYPE_REFERENCE,
            "diagnostic-schema-004-unknown-metadata-key",
            format!("projector '{projector}' references unknown metadata key '{key}'"),
            span,
            [
                (
                    "projector",
                    DiagnosticArgumentValue::String(projector.to_owned()),
                ),
                ("key", DiagnosticArgumentValue::String(key.to_owned())),
            ],
        ));
        return None;
    };
    if !metadata.targets.contains(&target) {
        diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-projection-metadata-target",
            format!(
                "projector '{projector}' references metadata key '{key}' on unsupported target '{}'",
                metadata_target_name(target)
            ),
            span,
            [
                ("projector", DiagnosticArgumentValue::String(projector.to_owned())),
                ("key", DiagnosticArgumentValue::String(key.to_owned())),
                ("target", DiagnosticArgumentValue::String(metadata_target_name(target).to_owned())),
            ],
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
