use std::collections::BTreeSet;

use super::super::super::diagnostics::DUPLICATE_DEFINITION;
use super::super::super::raw::{RawProjectionInput, RawProjectionInputSource};
use super::super::super::spans::ManifestSpans;
use super::super::super::validate::{PendingTypeReference, validate_manifest_name};
use super::super::types::lower_type_reference;
use super::candidate::{
    CandidateKind, lower_occurrence, validate_availability_reason_arg_source,
    validate_candidate_metadata_source, validate_candidate_source,
};
use super::literal::lower_literal_for_type;
use crate::Diagnostic;
use crate::schema::{
    ProjectSchema, ProjectionInput, SchemaLiteralValue, SchemaProjectionInputSource,
    SchemaProjectionSelector, SchemaTypeRef,
};

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
pub(super) fn lower_inputs(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    selector: &SchemaProjectionSelector,
    raw_inputs: Vec<RawProjectionInput>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) -> Vec<ProjectionInput> {
    let mut seen = BTreeSet::new();
    raw_inputs
        .into_iter()
        .map(|raw| {
            let name_span = spans.next_value_span(file, source, &raw.name);
            validate_manifest_name(
                diagnostics,
                "projection input name",
                &raw.name,
                name_span.clone(),
            );
            if !seen.insert(raw.name.clone()) {
                diagnostics.push(Diagnostic::error(
                    DUPLICATE_DEFINITION,
                    format!("projector '{projector}' repeats input '{}'", raw.name),
                    name_span,
                ));
            }

            let (type_ref, type_ref_span, type_ref_valid) = lower_type_reference(
                file,
                source,
                spans,
                diagnostics,
                &raw.type_ref,
                format!(
                    "projector '{projector}' input '{}' has invalid type reference '{}'",
                    raw.name, raw.type_ref
                ),
            );
            if type_ref_valid {
                pending_type_refs.push(PendingTypeReference {
                    owner: format!("projector '{projector}' input '{}'", raw.name),
                    type_ref: type_ref.clone(),
                    span: type_ref_span,
                });
            }

            let source = lower_input_source(
                file,
                source,
                spans,
                diagnostics,
                schema,
                projector,
                selector,
                &raw.name,
                &type_ref,
                raw.source,
            );
            ProjectionInput {
                name: raw.name,
                source,
                type_ref,
                required: raw.required,
            }
        })
        .collect()
}

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
fn lower_input_source(
    file: &str,
    source_text: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    selector: &SchemaProjectionSelector,
    input: &str,
    type_ref: &SchemaTypeRef,
    raw: RawProjectionInputSource,
) -> SchemaProjectionInputSource {
    match raw {
        RawProjectionInputSource::EventKind => SchemaProjectionInputSource::EventKind,
        RawProjectionInputSource::CandidateLineId => {
            validate_candidate_source(
                diagnostics,
                projector,
                input,
                selector,
                CandidateKind::Line,
                spans.next_key_span(file, source_text, input),
            );
            SchemaProjectionInputSource::CandidateLineId
        }
        RawProjectionInputSource::CandidateChoiceId => {
            validate_candidate_source(
                diagnostics,
                projector,
                input,
                selector,
                CandidateKind::Choice,
                spans.next_key_span(file, source_text, input),
            );
            SchemaProjectionInputSource::CandidateChoiceId
        }
        RawProjectionInputSource::CandidateEffectRequestId => {
            validate_candidate_source(
                diagnostics,
                projector,
                input,
                selector,
                CandidateKind::Effect,
                spans.next_key_span(file, source_text, input),
            );
            SchemaProjectionInputSource::CandidateEffectRequestId
        }
        RawProjectionInputSource::CandidateBlockId => {
            validate_candidate_source(
                diagnostics,
                projector,
                input,
                selector,
                CandidateKind::Block,
                spans.next_key_span(file, source_text, input),
            );
            SchemaProjectionInputSource::CandidateBlockId
        }
        RawProjectionInputSource::CandidateProject => {
            validate_candidate_source(
                diagnostics,
                projector,
                input,
                selector,
                CandidateKind::Project,
                spans.next_key_span(file, source_text, input),
            );
            SchemaProjectionInputSource::CandidateProject
        }
        RawProjectionInputSource::CandidateMetadata { key, occurrence } => {
            let key_span = spans.next_value_span(file, source_text, &key);
            let occurrence =
                lower_occurrence(diagnostics, projector, input, occurrence, key_span.clone());
            validate_candidate_metadata_source(
                diagnostics,
                schema,
                projector,
                input,
                selector,
                &key,
                &occurrence,
                type_ref,
                key_span,
            );
            SchemaProjectionInputSource::CandidateMetadata { key, occurrence }
        }
        RawProjectionInputSource::AvailabilityReasonArg { name } => {
            let name_span = spans.next_value_span(file, source_text, &name);
            validate_availability_reason_arg_source(
                diagnostics,
                schema,
                projector,
                input,
                selector,
                &name,
                type_ref,
                name_span,
            );
            SchemaProjectionInputSource::AvailabilityReasonArg { name }
        }
        RawProjectionInputSource::Literal { value } => {
            let literal = lower_literal_for_type(
                diagnostics,
                schema,
                &format!("projector '{projector}' input '{input}'"),
                type_ref,
                value,
                spans.next_key_span(file, source_text, input),
            )
            .unwrap_or_else(|| SchemaLiteralValue::String(String::new()));
            SchemaProjectionInputSource::Literal(literal)
        }
    }
}
