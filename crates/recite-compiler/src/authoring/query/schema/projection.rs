use recite_core::{ProjectSchema, SourceSpan};

use super::super::types::{
    CompletionCandidate, CompletionCandidateDetail, CompletionCandidateKind,
};

pub(super) fn candidates(
    schema: &ProjectSchema,
    projector: Option<&str>,
    span: &SourceSpan,
    output: &mut Vec<CompletionCandidate>,
) {
    output.extend(schema.projection_queries.iter().map(|(name, definition)| {
        CompletionCandidate::new(
            name.clone(),
            CompletionCandidateKind::ProjectionQuery,
            CompletionCandidateDetail::Projection {
                parameters: definition.params.len(),
            },
            span.clone(),
        )
    }));
    for (name, definition) in &schema.presentation_projectors {
        if projector.is_some_and(|projector| projector != name) {
            continue;
        }
        output.push(CompletionCandidate::new(
            name.clone(),
            CompletionCandidateKind::ProjectionProjector,
            CompletionCandidateDetail::None,
            span.clone(),
        ));
        output.extend(definition.inputs.iter().map(|input| {
            CompletionCandidate::new(
                input.name.clone(),
                CompletionCandidateKind::ProjectionInput,
                CompletionCandidateDetail::None,
                span.clone(),
            )
        }));
        output.extend(definition.queries.keys().map(|name| {
            CompletionCandidate::new(
                name.clone(),
                CompletionCandidateKind::ProjectionQueryResult,
                CompletionCandidateDetail::None,
                span.clone(),
            )
        }));
        output.extend(definition.outputs.keys().map(|name| {
            CompletionCandidate::new(
                name.clone(),
                CompletionCandidateKind::ProjectionOutput,
                CompletionCandidateDetail::None,
                span.clone(),
            )
        }));
        output.extend(
            definition
                .outputs
                .values()
                .filter_map(|output| output.label.as_ref())
                .map(|label| {
                    CompletionCandidate::new(
                        label.template_id.clone(),
                        CompletionCandidateKind::ProjectionLabel,
                        CompletionCandidateDetail::None,
                        span.clone(),
                    )
                }),
        );
    }
}
