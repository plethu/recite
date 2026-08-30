use recite_core::{
    MetadataDomainDefinition, MetadataTarget, ProjectSchema, SchemaTypeDefinition, SourceSpan,
};

use super::super::types::{
    CompletionCandidate, CompletionCandidateDetail, CompletionCandidateKind, QueryUnavailableReason,
};
use super::metadata_context::{SelectorResolution, empty_values, resolve_selector};

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
    unavailable: &mut Vec<QueryUnavailableReason>,
    output: &mut Vec<CompletionCandidate>,
) {
    let Some(definition) = schema.metadata.get(key) else {
        return;
    };
    // Block metadata is already a first-class authoring site in the source
    // language.  Keep completion useful there even for manifests produced
    // before block targets were recorded explicitly; validation remains the
    // authority for whether the authored key is accepted.
    if !definition.targets.contains(&target) && !matches!(target, MetadataTarget::Block) {
        return;
    }
    if let Some(domain) = definition
        .domain
        .as_ref()
        .and_then(|name| schema.metadata_domains.get(name))
    {
        let detail = CompletionCandidateDetail::Metadata {
            type_ref: definition.type_ref.clone(),
            domain: definition.domain.clone(),
        };
        match domain {
            MetadataDomainDefinition::Flat(domain) => {
                output.extend(domain.values.iter().map(|value| {
                    candidate(
                        value,
                        CompletionCandidateKind::MetadataValue,
                        detail.clone(),
                        span,
                    )
                }))
            }
            MetadataDomainDefinition::Contextual(domain) => {
                let context = resolve_selector(text, &domain.selector, span.start.line(), target);
                let values = match context {
                    SelectorResolution::Value(value) => domain
                        .values_by_context
                        .get(value)
                        .or_else(|| fallback_values(schema, &domain.missing_context, unavailable)),
                    SelectorResolution::Missing => match &domain.missing_context {
                        recite_core::MissingMetadataContextPolicy::Diagnostic => {
                            unavailable.push(QueryUnavailableReason::MissingMetadataContext);
                            None
                        }
                        recite_core::MissingMetadataContextPolicy::Empty => Some(empty_values()),
                        recite_core::MissingMetadataContextPolicy::Fallback { domain } => {
                            match schema.metadata_domains.get(domain) {
                                Some(MetadataDomainDefinition::Flat(domain)) => {
                                    Some(&domain.values)
                                }
                                Some(MetadataDomainDefinition::Contextual(_)) | None => {
                                    unavailable
                                        .push(QueryUnavailableReason::MalformedMetadataContext);
                                    None
                                }
                            }
                        }
                    },
                    SelectorResolution::Malformed => {
                        unavailable.push(QueryUnavailableReason::MalformedMetadataContext);
                        None
                    }
                };
                if let Some(values) = values {
                    output.extend(values.iter().map(|value| {
                        candidate(
                            value,
                            CompletionCandidateKind::MetadataValue,
                            detail.clone(),
                            span,
                        )
                    }));
                }
            }
        }
        return;
    }
    match &definition.type_ref {
        recite_core::SchemaTypeRef::Speaker => {
            output.extend(schema.speakers.iter().map(|(value, definition)| {
                candidate(
                    value,
                    CompletionCandidateKind::MetadataValue,
                    CompletionCandidateDetail::Speaker {
                        display_name: definition.display_name.clone(),
                    },
                    span,
                )
            }))
        }
        recite_core::SchemaTypeRef::Registry(name) => {
            if let Some(registry) = schema.registries.get(name) {
                output.extend(registry.values.iter().map(|value| {
                    candidate(
                        value,
                        CompletionCandidateKind::MetadataValue,
                        CompletionCandidateDetail::SchemaType(
                            recite_core::SchemaTypeRef::Registry(name.clone()),
                        ),
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
                        CompletionCandidateDetail::SchemaType(recite_core::SchemaTypeRef::Enum(
                            name.clone(),
                        )),
                        span,
                    )
                }));
            }
        }
        _ => {}
    }
}

fn fallback_values<'a>(
    schema: &'a ProjectSchema,
    policy: &recite_core::MissingMetadataContextPolicy,
    unavailable: &mut Vec<QueryUnavailableReason>,
) -> Option<&'a std::collections::BTreeSet<String>> {
    match policy {
        recite_core::MissingMetadataContextPolicy::Fallback { domain } => {
            match schema.metadata_domains.get(domain) {
                Some(MetadataDomainDefinition::Flat(domain)) => Some(&domain.values),
                Some(MetadataDomainDefinition::Contextual(_)) | None => {
                    unavailable.push(QueryUnavailableReason::MalformedMetadataContext);
                    None
                }
            }
        }
        recite_core::MissingMetadataContextPolicy::Diagnostic => {
            unavailable.push(QueryUnavailableReason::MissingMetadataContext);
            None
        }
        recite_core::MissingMetadataContextPolicy::Empty => Some(empty_values()),
    }
}
