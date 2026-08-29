use std::collections::{BTreeMap, BTreeSet};

use super::super::super::diagnostics::{DUPLICATE_DEFINITION, MALFORMED_SHAPE};
use super::super::super::raw::{Named, RawPresentationAffordanceOutputDefinition};
use super::super::super::validate::{validate_manifest_name, validate_non_empty_string};
use super::super::LoweringContext;
use super::field::lower_fields;
use super::label::lower_label;
use super::reference::query_result_types;
use super::{
    LabelContext, LabelIdState, OutputSources, PendingTypeRefs, ProjectionTypeTables,
    ProjectorContext,
};
use crate::schema::schema_diagnostic;
use crate::schema::{PresentationAffordanceOutputDefinition, ProjectionOutputTarget};
use crate::{Diagnostic, DiagnosticArgumentValue};

pub(super) fn lower_outputs(
    lowering: &mut LoweringContext<'_>,
    projector_context: ProjectorContext<'_>,
    sources: &OutputSources<'_>,
    raw_outputs: Vec<Named<RawPresentationAffordanceOutputDefinition>>,
    label_ids: &mut LabelIdState<'_>,
    pending_type_refs: &mut PendingTypeRefs<'_>,
    projector_path: &[String],
) -> BTreeMap<String, PresentationAffordanceOutputDefinition> {
    let mut seen = BTreeSet::new();
    let mut lowered = BTreeMap::new();
    let input_types = sources
        .inputs
        .iter()
        .map(|input| (input.name.as_str(), input.type_ref.clone()))
        .collect::<BTreeMap<_, _>>();
    let types = ProjectionTypeTables {
        input_types,
        query_types: query_result_types(projector_context.schema, sources.queries),
    };

    for raw_output in raw_outputs {
        let mut output_path = projector_path.to_vec();
        output_path.extend(["outputs".to_owned(), raw_output.name.clone()]);
        let output_span = lowering.key_span_at(&output_path, &raw_output.name);
        validate_manifest_name(
            lowering.diagnostics,
            "projection output id",
            &raw_output.name,
            output_span.clone(),
        );
        if !seen.insert(raw_output.name.clone()) {
            lowering.diagnostics.push(schema_diagnostic(
                DUPLICATE_DEFINITION,
                "diagnostic-schema-003-projection-output",
                format!(
                    "projector '{}' repeats output '{}'",
                    projector_context.projector, raw_output.name
                ),
                output_span,
                [
                    (
                        "projector",
                        DiagnosticArgumentValue::String(projector_context.projector.to_owned()),
                    ),
                    (
                        "output",
                        DiagnosticArgumentValue::String(raw_output.name.clone()),
                    ),
                ],
            ));
            continue;
        }
        let label_context = LabelContext {
            projector: projector_context.projector,
            output: &raw_output.name,
            types: &types,
        };
        let mut target_path = output_path.clone();
        target_path.push("target".to_owned());
        let target_span = lowering.value_span_at(&target_path, &raw_output.value.target);
        let target = lower_output_target(
            lowering.diagnostics,
            projector_context.projector,
            &raw_output.name,
            &raw_output.value.target,
            target_span,
        );
        let mut kind_path = output_path.clone();
        kind_path.push("kind".to_owned());
        let kind_span = lowering.value_span_at(&kind_path, &raw_output.value.kind);
        validate_non_empty_string(
            lowering.diagnostics,
            "projection output kind",
            &raw_output.value.kind,
            kind_span,
        );
        let mut slot_path = output_path.clone();
        slot_path.push("slot".to_owned());
        let slot_span = lowering.value_span_at(&slot_path, &raw_output.value.slot);
        validate_non_empty_string(
            lowering.diagnostics,
            "projection output slot",
            &raw_output.value.slot,
            slot_span,
        );
        let label = raw_output.value.label.map(|label| {
            lower_label(
                lowering,
                &label_context,
                label,
                label_ids,
                pending_type_refs,
                &output_path,
            )
        });
        let fields = lower_fields(
            lowering,
            &projector_context,
            &raw_output.name,
            raw_output.value.fields,
            &types,
            pending_type_refs,
            &output_path,
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
            diagnostics.push(schema_diagnostic(
                MALFORMED_SHAPE,
                "diagnostic-schema-001-projection-output-target",
                format!(
                    "projector '{projector}' output '{output}' uses unsupported target '{raw}'"
                ),
                span,
                [
                    (
                        "projector",
                        DiagnosticArgumentValue::String(projector.to_owned()),
                    ),
                    ("output", DiagnosticArgumentValue::String(output.to_owned())),
                    ("target", DiagnosticArgumentValue::String(raw.to_owned())),
                ],
            ));
            ProjectionOutputTarget::Candidate
        }
    }
}
