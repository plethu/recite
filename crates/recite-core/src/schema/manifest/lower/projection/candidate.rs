use super::super::super::diagnostics::{INVALID_TYPE_REFERENCE, MALFORMED_SHAPE};
use super::super::super::raw::RawMetadataOccurrence;
use super::selector::{selector_metadata_target, validate_metadata_key_target};
use super::{AvailabilityReasonContext, CandidateMetadataContext};
use crate::schema::schema_diagnostic;
use crate::schema::{MetadataOccurrence, SchemaProjectionSelector, SchemaTypeRef};
use crate::{Diagnostic, DiagnosticArgumentValue};

macro_rules! text_args {
    ($($name:literal => $value:expr),* $(,)?) => {
        [$(($name, DiagnosticArgumentValue::String($value.into()))),*]
    };
}

#[derive(Clone, Copy)]
pub(super) enum CandidateKind {
    Line,
    Choice,
    Effect,
    Block,
    Project,
}

pub(super) fn validate_candidate_source(
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    input: &str,
    selector: &SchemaProjectionSelector,
    candidate: CandidateKind,
    span: crate::SourceSpan,
) {
    let valid = match (selector, candidate) {
        (SchemaProjectionSelector::MetadataKey { target, .. }, CandidateKind::Line)
        | (SchemaProjectionSelector::MetadataSet { target, .. }, CandidateKind::Line) => {
            *target == crate::schema::MetadataTarget::Line
        }
        (SchemaProjectionSelector::MetadataKey { target, .. }, CandidateKind::Choice)
        | (SchemaProjectionSelector::MetadataSet { target, .. }, CandidateKind::Choice) => {
            *target == crate::schema::MetadataTarget::Choice
        }
        (SchemaProjectionSelector::MetadataKey { target, .. }, CandidateKind::Block)
        | (SchemaProjectionSelector::MetadataSet { target, .. }, CandidateKind::Block) => {
            *target == crate::schema::MetadataTarget::Block
        }
        (SchemaProjectionSelector::MetadataKey { target, .. }, CandidateKind::Project)
        | (SchemaProjectionSelector::MetadataSet { target, .. }, CandidateKind::Project) => {
            *target == crate::schema::MetadataTarget::Project
        }
        (SchemaProjectionSelector::RuntimeEvent { kind }, CandidateKind::Effect) => {
            kind == "effect"
        }
        (SchemaProjectionSelector::RuntimeEvent { .. }, _) => true,
        _ => false,
    };
    if !valid {
        diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-projection-candidate-source",
            format!(
                "projector '{projector}' input '{input}' uses an incompatible candidate id source"
            ),
            span,
            text_args!("projector" => projector, "input" => input),
        ));
    }
}

pub(super) fn validate_candidate_metadata_source(
    diagnostics: &mut Vec<Diagnostic>,
    context: CandidateMetadataContext<'_>,
    span: crate::SourceSpan,
) {
    let Some(target) = selector_metadata_target(context.selector) else {
        diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-projection-candidate-no-target",
            format!("projector '{}' input '{}' reads candidate metadata but its selector has no metadata target", context.projector, context.input),
            span,
            text_args!("projector" => context.projector, "input" => context.input),
        ));
        return;
    };
    let Some(metadata) = validate_metadata_key_target(
        diagnostics,
        context.schema,
        context.projector,
        context.key,
        target,
        span.clone(),
    ) else {
        return;
    };
    if !metadata.repeatable && !matches!(context.occurrence, MetadataOccurrence::Only) {
        diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-projection-occurrence-repeat",
            format!("projector '{}' input '{}' uses repeated occurrence '{}' for non-repeatable metadata key '{}'", context.projector, context.input, occurrence_name(context.occurrence), context.key),
            span.clone(),
            text_args!("projector" => context.projector, "input" => context.input, "occurrence" => occurrence_name(context.occurrence), "key" => context.key),
        ));
    }
    match context.occurrence {
        MetadataOccurrence::All => {
            let SchemaTypeRef::Array(inner) = context.type_ref else {
                diagnostics.push(schema_diagnostic(
                    MALFORMED_SHAPE,
                    "diagnostic-schema-001-projection-occurrence-all-type",
                    format!("projector '{}' input '{}' uses occurrence 'all' but has non-array type {}", context.projector, context.input, super::reference::type_ref_name(context.type_ref)),
                    span,
                    text_args!("projector" => context.projector, "input" => context.input, "type_ref" => super::reference::type_ref_name(context.type_ref)),
                ));
                return;
            };
            if **inner != metadata.type_ref {
                diagnostics.push(schema_diagnostic(
                    MALFORMED_SHAPE,
                    "diagnostic-schema-001-projection-candidate-type-mismatch",
                    format!("projector '{}' input '{}' expects {}, but metadata key '{}' has {}", context.projector, context.input, super::reference::type_ref_name(context.type_ref), context.key, super::reference::type_ref_name(&metadata.type_ref)),
                    span,
                    text_args!("projector" => context.projector, "input" => context.input, "expected" => super::reference::type_ref_name(context.type_ref), "key" => context.key, "actual" => super::reference::type_ref_name(&metadata.type_ref)),
                ));
            }
        }
        _ if matches!(context.type_ref, SchemaTypeRef::Array(_)) => diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-projection-occurrence-array",
            format!("projector '{}' input '{}' uses array type {} without occurrence 'all'", context.projector, context.input, super::reference::type_ref_name(context.type_ref)),
            span,
            text_args!("projector" => context.projector, "input" => context.input, "type_ref" => super::reference::type_ref_name(context.type_ref)),
        )),
        _ if *context.type_ref != metadata.type_ref => diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-projection-candidate-type-mismatch",
            format!("projector '{}' input '{}' expects {}, but metadata key '{}' has {}", context.projector, context.input, super::reference::type_ref_name(context.type_ref), context.key, super::reference::type_ref_name(&metadata.type_ref)),
            span,
            text_args!("projector" => context.projector, "input" => context.input, "expected" => super::reference::type_ref_name(context.type_ref), "key" => context.key, "actual" => super::reference::type_ref_name(&metadata.type_ref)),
        )),
        _ => {}
    }
}

pub(super) fn validate_availability_reason_arg_source(
    diagnostics: &mut Vec<Diagnostic>,
    context: AvailabilityReasonContext<'_>,
    span: crate::SourceSpan,
) {
    let SchemaProjectionSelector::AvailabilityReason { reason_id } = context.selector else {
        diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-projection-reason-no-selector",
            format!("projector '{}' input '{}' reads an availability reason argument but its selector is not availability_reason", context.projector, context.input),
            span,
            text_args!("projector" => context.projector, "input" => context.input),
        ));
        return;
    };
    let Some(reason) = context.schema.availability_reasons.get(reason_id) else {
        return;
    };
    let Some(param) = reason
        .params
        .iter()
        .find(|param| param.name == context.name)
    else {
        diagnostics.push(schema_diagnostic(
            INVALID_TYPE_REFERENCE,
            "diagnostic-schema-004-projection-reason-arg",
            format!(
                "projector '{}' input '{}' references unknown availability reason argument '{}'",
                context.projector, context.input, context.name
            ),
            span,
            text_args!("projector" => context.projector, "input" => context.input, "name" => context.name),
        ));
        return;
    };
    if &param.type_ref != context.type_ref {
        diagnostics.push(schema_diagnostic(
            MALFORMED_SHAPE,
            "diagnostic-schema-001-projection-reason-type",
            format!(
                "projector '{}' input '{}' expects {}, but availability reason argument '{}' has {}",
                context.projector, context.input, super::reference::type_ref_name(context.type_ref),
                context.name,
                super::reference::type_ref_name(&param.type_ref)
            ),
            span,
            text_args!("projector" => context.projector, "input" => context.input, "name" => context.name, "expected" => super::reference::type_ref_name(context.type_ref), "actual" => super::reference::type_ref_name(&param.type_ref)),
        ));
    }
}

pub(super) fn lower_occurrence(
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    input: &str,
    raw: Option<RawMetadataOccurrence>,
    span: crate::SourceSpan,
) -> MetadataOccurrence {
    match raw {
        None => MetadataOccurrence::Only,
        Some(RawMetadataOccurrence::Named(name)) if name == "only" => MetadataOccurrence::Only,
        Some(RawMetadataOccurrence::Named(name)) if name == "first" => MetadataOccurrence::First,
        Some(RawMetadataOccurrence::Named(name)) if name == "last" => MetadataOccurrence::Last,
        Some(RawMetadataOccurrence::Named(name)) if name == "all" => MetadataOccurrence::All,
        Some(RawMetadataOccurrence::Index(index)) => MetadataOccurrence::Index(index.index),
        Some(RawMetadataOccurrence::Named(name)) => {
            diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-projection-occurrence",
                format!(
                    "projector '{projector}' input '{input}' uses unsupported metadata occurrence '{name}'"
                ),
                span,
                text_args!("projector" => projector, "input" => input, "name" => name),
            ));
            MetadataOccurrence::Only
        }
    }
}

fn occurrence_name(occurrence: &MetadataOccurrence) -> &'static str {
    match occurrence {
        MetadataOccurrence::Only => "only",
        MetadataOccurrence::First => "first",
        MetadataOccurrence::Last => "last",
        MetadataOccurrence::Index(_) => "index",
        MetadataOccurrence::All => "all",
    }
}
