use recite_core::{
    MetadataContextSelector, MetadataDomainDefinition, MetadataTarget, ProjectSchema,
    SchemaTypeDefinition, SourceSpan,
};

use super::super::types::{
    CompletionCandidate, CompletionCandidateDetail, CompletionCandidateKind,
};

fn candidate(
    name: &str,
    kind: CompletionCandidateKind,
    detail: CompletionCandidateDetail,
    span: &SourceSpan,
) -> CompletionCandidate {
    CompletionCandidate::new(name.to_owned(), kind, detail, span.clone())
}

pub(super) fn speaker_candidates(
    schema: &ProjectSchema,
    kind: CompletionCandidateKind,
    span: &SourceSpan,
    output: &mut Vec<CompletionCandidate>,
) {
    output.extend(schema.speakers.iter().map(|(name, definition)| {
        candidate(
            name,
            kind,
            CompletionCandidateDetail::Speaker {
                display_name: definition.display_name.clone(),
            },
            span,
        )
    }));
}

pub(super) fn key_candidates(
    schema: &ProjectSchema,
    kind: CompletionCandidateKind,
    target: MetadataTarget,
    span: &SourceSpan,
    output: &mut Vec<CompletionCandidate>,
) {
    output.extend(
        schema
            .metadata
            .iter()
            .filter(|(_, definition)| definition.targets.contains(&target))
            .map(|(name, definition)| {
                candidate(
                    name,
                    kind,
                    CompletionCandidateDetail::Metadata {
                        type_ref: definition.type_ref.clone(),
                        domain: definition.domain.clone(),
                    },
                    span,
                )
            }),
    );
}

pub(super) fn condition_candidates(
    schema: &ProjectSchema,
    kind: CompletionCandidateKind,
    span: &SourceSpan,
    output: &mut Vec<CompletionCandidate>,
) {
    output.extend(schema.conditions.iter().map(|(name, definition)| {
        candidate(
            name,
            kind,
            CompletionCandidateDetail::Parameters(definition.params.len()),
            span,
        )
    }));
}

pub(super) fn effect_candidates(
    schema: &ProjectSchema,
    kind: CompletionCandidateKind,
    span: &SourceSpan,
    output: &mut Vec<CompletionCandidate>,
) {
    output.extend(schema.effects.iter().map(|(name, definition)| {
        candidate(
            name,
            kind,
            CompletionCandidateDetail::Parameters(definition.params.len()),
            span,
        )
    }));
}

pub(super) fn reason_candidates(
    schema: &ProjectSchema,
    kind: CompletionCandidateKind,
    span: &SourceSpan,
    output: &mut Vec<CompletionCandidate>,
) {
    output.extend(
        schema
            .availability_reasons
            .iter()
            .map(|(name, definition)| {
                candidate(
                    name.as_str(),
                    kind,
                    CompletionCandidateDetail::AvailabilityReason {
                        template: definition.template.clone(),
                        parameters: definition.params.len(),
                    },
                    span,
                )
            }),
    );
}

pub(super) fn value_candidates(
    schema: &ProjectSchema,
    text: &str,
    key: &str,
    target: MetadataTarget,
    span: &SourceSpan,
    output: &mut Vec<CompletionCandidate>,
) {
    let Some(definition) = schema.metadata.get(key) else {
        return;
    };
    if !definition.targets.contains(&target) {
        return;
    }
    if let Some(domain) = definition
        .domain
        .as_ref()
        .and_then(|name| schema.metadata_domains.get(name))
    {
        match domain {
            MetadataDomainDefinition::Flat(domain) => {
                output.extend(domain.values.iter().map(|value| {
                    candidate(
                        value,
                        CompletionCandidateKind::MetadataValue,
                        CompletionCandidateDetail::None,
                        span,
                    )
                }))
            }
            MetadataDomainDefinition::Contextual(domain) => {
                if let Some(context) = selector_value(text, &domain.selector, span.start.line())
                    .and_then(|value| domain.values_by_context.get(value))
                {
                    output.extend(context.iter().map(|value| {
                        candidate(
                            value,
                            CompletionCandidateKind::MetadataValue,
                            CompletionCandidateDetail::None,
                            span,
                        )
                    }));
                }
            }
        }
        return;
    }
    match &definition.type_ref {
        recite_core::SchemaTypeRef::Speaker => output.extend(schema.speakers.keys().map(|value| {
            candidate(
                value,
                CompletionCandidateKind::MetadataValue,
                CompletionCandidateDetail::None,
                span,
            )
        })),
        recite_core::SchemaTypeRef::Registry(name) => {
            if let Some(registry) = schema.registries.get(name) {
                output.extend(registry.values.iter().map(|value| {
                    candidate(
                        value,
                        CompletionCandidateKind::MetadataValue,
                        CompletionCandidateDetail::None,
                        span,
                    )
                }));
            }
        }
        recite_core::SchemaTypeRef::Enum(name) => {
            if let Some(SchemaTypeDefinition::Enum(definition)) = schema.types.get(name) {
                output.extend(definition.values.iter().map(|value| {
                    candidate(
                        value,
                        CompletionCandidateKind::MetadataValue,
                        CompletionCandidateDetail::None,
                        span,
                    )
                }));
            }
        }
        _ => {}
    }
}

fn selector_value<'a>(
    text: &'a str,
    selector: &MetadataContextSelector,
    line_number: u32,
) -> Option<&'a str> {
    let line = text.lines().nth(line_number.checked_sub(1)? as usize)?;
    let wanted = match selector {
        MetadataContextSelector::FieldSpeaker => "speaker",
        MetadataContextSelector::MetadataKey(key) => key,
    };
    recite_parser::metadata_assignments(line)
        .into_iter()
        .rev()
        .find(|assignment| assignment.key == wanted)
        .map(|assignment| assignment.value)
}
