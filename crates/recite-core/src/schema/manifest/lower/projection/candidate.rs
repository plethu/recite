use super::super::super::diagnostics::{INVALID_TYPE_REFERENCE, MALFORMED_SHAPE};
use super::super::super::raw::RawMetadataOccurrence;
use super::selector::{selector_metadata_target, validate_metadata_key_target};
use crate::Diagnostic;
use crate::schema::{MetadataOccurrence, ProjectSchema, SchemaProjectionSelector, SchemaTypeRef};

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
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!(
                "projector '{projector}' input '{input}' uses an incompatible candidate id source"
            ),
            span,
        ));
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "projection metadata validation carries selector, schema, type, and span context"
)]
pub(super) fn validate_candidate_metadata_source(
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    input: &str,
    selector: &SchemaProjectionSelector,
    key: &str,
    occurrence: &MetadataOccurrence,
    type_ref: &SchemaTypeRef,
    span: crate::SourceSpan,
) {
    let Some(target) = selector_metadata_target(selector) else {
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!("projector '{projector}' input '{input}' reads candidate metadata but its selector has no metadata target"),
            span,
        ));
        return;
    };
    let Some(metadata) =
        validate_metadata_key_target(diagnostics, schema, projector, key, target, span.clone())
    else {
        return;
    };
    if !metadata.repeatable && !matches!(occurrence, MetadataOccurrence::Only) {
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!("projector '{projector}' input '{input}' uses repeated occurrence '{}' for non-repeatable metadata key '{key}'", occurrence_name(occurrence)),
            span.clone(),
        ));
    }
    match occurrence {
        MetadataOccurrence::All => {
            let SchemaTypeRef::Array(inner) = type_ref else {
                diagnostics.push(Diagnostic::error(
                    MALFORMED_SHAPE,
                    format!("projector '{projector}' input '{input}' uses occurrence 'all' but has non-array type {}", super::reference::type_ref_name(type_ref)),
                    span,
                ));
                return;
            };
            if **inner != metadata.type_ref {
                diagnostics.push(Diagnostic::error(
                    MALFORMED_SHAPE,
                    format!("projector '{projector}' input '{input}' expects {}, but metadata key '{key}' has {}", super::reference::type_ref_name(type_ref), super::reference::type_ref_name(&metadata.type_ref)),
                    span,
                ));
            }
        }
        _ if matches!(type_ref, SchemaTypeRef::Array(_)) => diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!("projector '{projector}' input '{input}' uses array type {} without occurrence 'all'", super::reference::type_ref_name(type_ref)),
            span,
        )),
        _ if *type_ref != metadata.type_ref => diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!("projector '{projector}' input '{input}' expects {}, but metadata key '{key}' has {}", super::reference::type_ref_name(type_ref), super::reference::type_ref_name(&metadata.type_ref)),
            span,
        )),
        _ => {}
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "availability projection validation carries selector, schema, type, and span context"
)]
pub(super) fn validate_availability_reason_arg_source(
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    input: &str,
    selector: &SchemaProjectionSelector,
    name: &str,
    type_ref: &SchemaTypeRef,
    span: crate::SourceSpan,
) {
    let SchemaProjectionSelector::AvailabilityReason { reason_id } = selector else {
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!("projector '{projector}' input '{input}' reads an availability reason argument but its selector is not availability_reason"),
            span,
        ));
        return;
    };
    let Some(reason) = schema.availability_reasons.get(reason_id) else {
        return;
    };
    let Some(param) = reason.params.iter().find(|param| param.name == name) else {
        diagnostics.push(Diagnostic::error(
            INVALID_TYPE_REFERENCE,
            format!(
                "projector '{projector}' input '{input}' references unknown availability reason argument '{name}'"
            ),
            span,
        ));
        return;
    };
    if &param.type_ref != type_ref {
        diagnostics.push(Diagnostic::error(
            MALFORMED_SHAPE,
            format!(
                "projector '{projector}' input '{input}' expects {}, but availability reason argument '{name}' has {}",
                super::reference::type_ref_name(type_ref),
                super::reference::type_ref_name(&param.type_ref)
            ),
            span,
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
        Some(RawMetadataOccurrence::Index { index }) => MetadataOccurrence::Index(index),
        Some(RawMetadataOccurrence::Named(name)) => {
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "projector '{projector}' input '{input}' uses unsupported metadata occurrence '{name}'"
                ),
                span,
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
