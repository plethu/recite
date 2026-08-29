use std::collections::BTreeSet;

use super::super::super::diagnostics::DUPLICATE_DEFINITION;
use super::super::super::raw::{RawProjectionInput, RawProjectionInputSource};
use super::super::super::validate::{PendingTypeReference, validate_manifest_name};
use super::super::LoweringContext;
use super::candidate::{
    CandidateKind, lower_occurrence, validate_availability_reason_arg_source,
    validate_candidate_metadata_source, validate_candidate_source,
};
use super::literal::lower_literal_for_type;
use super::{AvailabilityReasonContext, CandidateMetadataContext, InputSourceContext};
use crate::DiagnosticArgumentValue;
use crate::schema::schema_diagnostic;
use crate::schema::{
    ProjectSchema, ProjectionInput, SchemaLiteralValue, SchemaProjectionInputSource,
    SchemaProjectionSelector,
};

pub(super) fn lower_inputs(
    lowering: &mut LoweringContext<'_>,
    schema: &ProjectSchema,
    projector: &str,
    selector: &SchemaProjectionSelector,
    raw_inputs: Vec<RawProjectionInput>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
    projector_path: &[String],
) -> Vec<ProjectionInput> {
    let mut seen = BTreeSet::new();
    raw_inputs
        .into_iter()
        .enumerate()
        .map(|(index, raw)| {
            let mut input_path = projector_path.to_vec();
            input_path.extend(["inputs".to_owned(), format!("[{index}]")]);
            let mut name_path = input_path.clone();
            name_path.push("name".to_owned());
            let name_span = lowering.value_span_at(&name_path, &raw.name);
            validate_manifest_name(
                lowering.diagnostics,
                "projection input name",
                &raw.name,
                name_span.clone(),
            );
            if !seen.insert(raw.name.clone()) {
                lowering.diagnostics.push(schema_diagnostic(
                    DUPLICATE_DEFINITION,
                    "diagnostic-schema-003-projection-input",
                    format!("projector '{projector}' repeats input '{}'", raw.name),
                    name_span,
                    [
                        (
                            "projector",
                            DiagnosticArgumentValue::String(projector.to_owned()),
                        ),
                        ("input", DiagnosticArgumentValue::String(raw.name.clone())),
                    ],
                ));
            }
            let mut type_path = input_path.clone();
            type_path.push("type".to_owned());
            let (type_ref, type_ref_span, type_ref_valid) =
                super::super::types::lower_type_reference_at_with_context(
                    lowering,
                    &raw.type_ref,
                    &type_path,
                    format!(
                        "projector '{projector}' input '{}' has invalid type reference '{}'",
                        raw.name, raw.type_ref
                    ),
                    super::super::types::TypeReferenceContext::ProjectionInput {
                        projector: projector.to_owned(),
                        input: raw.name.clone(),
                    },
                );
            if type_ref_valid {
                pending_type_refs.push(PendingTypeReference {
                    owner: format!("projector '{projector}' input '{}'", raw.name),
                    type_ref: type_ref.clone(),
                    span: type_ref_span,
                });
            }
            let source = lower_input_source(
                lowering,
                InputSourceContext {
                    schema,
                    projector,
                    selector,
                    input: &raw.name,
                    type_ref: &type_ref,
                },
                raw.source,
                &input_path,
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
fn lower_input_source(
    lowering: &mut LoweringContext<'_>,
    context: InputSourceContext<'_>,
    raw: RawProjectionInputSource,
    input_path: &[String],
) -> SchemaProjectionInputSource {
    match raw {
        RawProjectionInputSource::EventKind => SchemaProjectionInputSource::EventKind,
        RawProjectionInputSource::CandidateLineId => {
            let mut path = input_path.to_vec();
            path.extend(["source".to_owned(), "kind".to_owned()]);
            let span = lowering.value_span_at(&path, "candidate_line_id");
            validate_candidate_source(
                lowering.diagnostics,
                context.projector,
                context.input,
                context.selector,
                CandidateKind::Line,
                span,
            );
            SchemaProjectionInputSource::CandidateLineId
        }
        RawProjectionInputSource::CandidateChoiceId => {
            let mut path = input_path.to_vec();
            path.extend(["source".to_owned(), "kind".to_owned()]);
            let span = lowering.value_span_at(&path, "candidate_choice_id");
            validate_candidate_source(
                lowering.diagnostics,
                context.projector,
                context.input,
                context.selector,
                CandidateKind::Choice,
                span,
            );
            SchemaProjectionInputSource::CandidateChoiceId
        }
        RawProjectionInputSource::CandidateEffectRequestId => {
            let mut path = input_path.to_vec();
            path.extend(["source".to_owned(), "kind".to_owned()]);
            let span = lowering.value_span_at(&path, "candidate_effect_request_id");
            validate_candidate_source(
                lowering.diagnostics,
                context.projector,
                context.input,
                context.selector,
                CandidateKind::Effect,
                span,
            );
            SchemaProjectionInputSource::CandidateEffectRequestId
        }
        RawProjectionInputSource::CandidateBlockId => {
            let mut path = input_path.to_vec();
            path.extend(["source".to_owned(), "kind".to_owned()]);
            let span = lowering.value_span_at(&path, "candidate_block_id");
            validate_candidate_source(
                lowering.diagnostics,
                context.projector,
                context.input,
                context.selector,
                CandidateKind::Block,
                span,
            );
            SchemaProjectionInputSource::CandidateBlockId
        }
        RawProjectionInputSource::CandidateProject => {
            let mut path = input_path.to_vec();
            path.extend(["source".to_owned(), "kind".to_owned()]);
            let span = lowering.value_span_at(&path, "candidate_project");
            validate_candidate_source(
                lowering.diagnostics,
                context.projector,
                context.input,
                context.selector,
                CandidateKind::Project,
                span,
            );
            SchemaProjectionInputSource::CandidateProject
        }
        RawProjectionInputSource::CandidateMetadata { key, occurrence } => {
            let mut key_path = input_path.to_vec();
            key_path.extend(["source".to_owned(), "key".to_owned()]);
            let key_span = lowering.value_span_at(&key_path, &key);
            let occurrence = lower_occurrence(
                lowering.diagnostics,
                context.projector,
                context.input,
                occurrence,
                key_span.clone(),
            );
            validate_candidate_metadata_source(
                lowering.diagnostics,
                CandidateMetadataContext {
                    schema: context.schema,
                    projector: context.projector,
                    input: context.input,
                    selector: context.selector,
                    key: &key,
                    occurrence: &occurrence,
                    type_ref: context.type_ref,
                },
                key_span,
            );
            SchemaProjectionInputSource::CandidateMetadata { key, occurrence }
        }
        RawProjectionInputSource::AvailabilityReasonArg { name } => {
            let mut name_path = input_path.to_vec();
            name_path.extend(["source".to_owned(), "name".to_owned()]);
            let name_span = lowering.value_span_at(&name_path, &name);
            validate_availability_reason_arg_source(
                lowering.diagnostics,
                AvailabilityReasonContext {
                    schema: context.schema,
                    projector: context.projector,
                    input: context.input,
                    selector: context.selector,
                    name: &name,
                    type_ref: context.type_ref,
                },
                name_span,
            );
            SchemaProjectionInputSource::AvailabilityReasonArg { name }
        }
        RawProjectionInputSource::Literal { value } => {
            let mut value_path = input_path.to_vec();
            value_path.extend(["source".to_owned(), "value".to_owned()]);
            let literal_span = lowering.value_span_at(&value_path, "literal");
            let literal = lower_literal_for_type(
                lowering.diagnostics,
                context.schema,
                &format!(
                    "projector '{}' input '{}'",
                    context.projector, context.input
                ),
                context.type_ref,
                value,
                literal_span,
            )
            .unwrap_or_else(|| SchemaLiteralValue::String(String::new()));
            SchemaProjectionInputSource::Literal(literal)
        }
    }
}
