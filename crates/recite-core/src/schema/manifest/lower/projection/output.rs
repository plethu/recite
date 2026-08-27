use std::collections::{BTreeMap, BTreeSet};

use super::super::super::diagnostics::{DUPLICATE_DEFINITION, MALFORMED_SHAPE};
use super::super::super::raw::{Named, RawPresentationAffordanceOutputDefinition};
use super::super::super::spans::ManifestSpans;
use super::super::super::validate::{
    PendingTypeReference, validate_manifest_name, validate_non_empty_string,
};
use super::field::lower_fields;
use super::label::lower_label;
use super::reference::query_result_types;
use crate::Diagnostic;
use crate::schema::{
    PresentationAffordanceOutputDefinition, ProjectSchema, ProjectionInput, ProjectionOutputTarget,
    ProjectionQueryDefinition,
};

#[expect(
    clippy::too_many_arguments,
    reason = "manifest lowering helpers carry shared JSON span, schema, and diagnostic context"
)]
pub(super) fn lower_outputs(
    file: &str,
    source: &str,
    spans: &mut ManifestSpans,
    diagnostics: &mut Vec<Diagnostic>,
    schema: &ProjectSchema,
    projector: &str,
    inputs: &[ProjectionInput],
    queries: &BTreeMap<String, ProjectionQueryDefinition>,
    raw_outputs: Vec<Named<RawPresentationAffordanceOutputDefinition>>,
    seen_label_ids: &mut BTreeSet<String>,
    pending_type_refs: &mut Vec<PendingTypeReference>,
) -> BTreeMap<String, PresentationAffordanceOutputDefinition> {
    let mut seen = BTreeSet::new();
    let mut lowered = BTreeMap::new();
    let input_types = inputs
        .iter()
        .map(|input| (input.name.as_str(), input.type_ref.clone()))
        .collect::<BTreeMap<_, _>>();
    let query_types = query_result_types(schema, queries);

    for raw_output in raw_outputs {
        let output_span = spans.next_key_span(file, source, &raw_output.name);
        validate_manifest_name(
            diagnostics,
            "projection output id",
            &raw_output.name,
            output_span.clone(),
        );
        if !seen.insert(raw_output.name.clone()) {
            diagnostics.push(Diagnostic::error(
                DUPLICATE_DEFINITION,
                format!(
                    "projector '{projector}' repeats output '{}'",
                    raw_output.name
                ),
                output_span,
            ));
            continue;
        }
        let target = lower_output_target(
            diagnostics,
            projector,
            &raw_output.name,
            &raw_output.value.target,
            spans.next_value_span(file, source, &raw_output.value.target),
        );
        let kind_span = spans.next_value_span(file, source, &raw_output.value.kind);
        validate_non_empty_string(
            diagnostics,
            "projection output kind",
            &raw_output.value.kind,
            kind_span,
        );
        let slot_span = spans.next_value_span(file, source, &raw_output.value.slot);
        validate_non_empty_string(
            diagnostics,
            "projection output slot",
            &raw_output.value.slot,
            slot_span,
        );
        let label = raw_output.value.label.map(|label| {
            lower_label(
                file,
                source,
                spans,
                diagnostics,
                projector,
                &raw_output.name,
                label,
                &input_types,
                &query_types,
                seen_label_ids,
                pending_type_refs,
            )
        });
        let fields = lower_fields(
            file,
            source,
            spans,
            diagnostics,
            projector,
            &raw_output.name,
            raw_output.value.fields,
            schema,
            &input_types,
            &query_types,
            pending_type_refs,
        );
        lowered.insert(
            raw_output.name,
            PresentationAffordanceOutputDefinition {
                target,
                kind: raw_output.value.kind,
                slot: raw_output.value.slot,
                label,
                fields,
            },
        );
    }

    lowered
}

fn lower_output_target(
    diagnostics: &mut Vec<Diagnostic>,
    projector: &str,
    output: &str,
    raw: &str,
    span: crate::SourceSpan,
) -> ProjectionOutputTarget {
    match raw {
        "candidate" => ProjectionOutputTarget::Candidate,
        "event" => ProjectionOutputTarget::Event,
        "prompt" => ProjectionOutputTarget::Prompt,
        _ => {
            diagnostics.push(Diagnostic::error(
                MALFORMED_SHAPE,
                format!(
                    "projector '{projector}' output '{output}' uses unsupported target '{raw}'"
                ),
                span,
            ));
            ProjectionOutputTarget::Candidate
        }
    }
}
