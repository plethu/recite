use std::collections::BTreeSet;

use super::super::super::diagnostics::DUPLICATE_DEFINITION;
use super::super::super::raw::{RawProjectionInput, RawProjectionInputSource};
use super::super::super::spans::ManifestSpans;
use super::super::super::validate::{PendingTypeReference, validate_manifest_name};
use super::candidate::{
    CandidateKind, lower_occurrence, validate_availability_reason_arg_source,
    validate_candidate_metadata_source, validate_candidate_source,
};
use super::literal::lower_literal_for_type;
use crate::schema::schema_diagnostic;
use crate::schema::{
    ProjectSchema, ProjectionInput, SchemaLiteralValue, SchemaProjectionInputSource,
    SchemaProjectionSelector, SchemaTypeRef,
};
use crate::{Diagnostic, DiagnosticArgumentValue};

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
            let name_span = spans.value_span_at(file, source, &name_path, &raw.name);
            validate_manifest_name(
                diagnostics,
                "projection input name",
                &raw.name,
                name_span.clone(),
            );
            if !seen.insert(raw.name.clone()) {
                diagnostics.push(schema_diagnostic(
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
                    file,
                    source,
                    spans,
                    diagnostics,
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
    input_path: &[String],
) -> SchemaProjectionInputSource {
    match raw {
        RawProjectionInputSource::EventKind => SchemaProjectionInputSource::EventKind,
        RawProjectionInputSource::CandidateLineId => {
            let mut path = input_path.to_vec();
            path.extend(["source".to_owned(), "kind".to_owned()]);
            validate_candidate_source(
                diagnostics,
                projector,
                input,
                selector,
                CandidateKind::Line,
                spans.value_span_at(file, source_text, &path, "candidate_line_id"),
            );
            SchemaProjectionInputSource::CandidateLineId
        }
        RawProjectionInputSource::CandidateChoiceId => {
            let mut path = input_path.to_vec();
            path.extend(["source".to_owned(), "kind".to_owned()]);
            validate_candidate_source(
                diagnostics,
                projector,
                input,
                selector,
                CandidateKind::Choice,
                spans.value_span_at(file, source_text, &path, "candidate_choice_id"),
            );
            SchemaProjectionInputSource::CandidateChoiceId
        }
        RawProjectionInputSource::CandidateEffectRequestId => {
            let mut path = input_path.to_vec();
            path.extend(["source".to_owned(), "kind".to_owned()]);
            validate_candidate_source(
                diagnostics,
                projector,
                input,
                selector,
                CandidateKind::Effect,
                spans.value_span_at(file, source_text, &path, "candidate_effect_request_id"),
            );
            SchemaProjectionInputSource::CandidateEffectRequestId
        }
        RawProjectionInputSource::CandidateBlockId => {
            let mut path = input_path.to_vec();
            path.extend(["source".to_owned(), "kind".to_owned()]);
            validate_candidate_source(
                diagnostics,
                projector,
                input,
                selector,
                CandidateKind::Block,
                spans.value_span_at(file, source_text, &path, "candidate_block_id"),
            );
            SchemaProjectionInputSource::CandidateBlockId
        }
        RawProjectionInputSource::CandidateProject => {
            let mut path = input_path.to_vec();
            path.extend(["source".to_owned(), "kind".to_owned()]);
            validate_candidate_source(
                diagnostics,
                projector,
                input,
                selector,
                CandidateKind::Project,
                spans.value_span_at(file, source_text, &path, "candidate_project"),
            );
            SchemaProjectionInputSource::CandidateProject
        }
        RawProjectionInputSource::CandidateMetadata { key, occurrence } => {
            let mut key_path = input_path.to_vec();
            key_path.extend(["source".to_owned(), "key".to_owned()]);
            let key_span = spans.value_span_at(file, source_text, &key_path, &key);
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
            let mut name_path = input_path.to_vec();
            name_path.extend(["source".to_owned(), "name".to_owned()]);
            let name_span = spans.value_span_at(file, source_text, &name_path, &name);
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
                {
                    let mut value_path = input_path.to_vec();
                    value_path.extend(["source".to_owned(), "value".to_owned()]);
                    spans.value_span_at(file, source_text, &value_path, "literal")
                },
            )
            .unwrap_or_else(|| SchemaLiteralValue::String(String::new()));
            SchemaProjectionInputSource::Literal(literal)
        }
    }
}
