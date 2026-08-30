use recite_core::{DocumentKey, SourcePosition};

use super::super::snapshot::AuthoringSnapshot;
use super::types::{QueryClass, QueryResult, QueryUnavailableReason};

impl AuthoringSnapshot {
    /// Returns typed candidates for a caller-provided language context.
    #[must_use]
    pub fn complete(
        &self,
        key: &DocumentKey,
        position: SourcePosition,
        context: super::types::CompletionContext,
    ) -> QueryResult<Vec<super::types::CompletionCandidate>> {
        let Some(document) = self.document(key) else {
            return QueryResult::NoMatch;
        };
        let replace_span = recite_core::SourceSpan::point(key.as_str(), position);
        let mut candidates = Vec::new();
        match context {
            super::types::CompletionContext::Blocks { document: target } => {
                if !document.participation().block_references().is_complete() {
                    return QueryResult::Unavailable(QueryUnavailableReason::Incomplete(
                        QueryClass::BlockReferences,
                    ));
                }
                for target_document in self.documents() {
                    if target
                        .as_ref()
                        .is_some_and(|target| target != target_document.key())
                    {
                        continue;
                    }
                    if !target_document
                        .participation()
                        .block_definitions()
                        .is_complete()
                    {
                        continue;
                    }
                    for block in target_document.summary().blocks() {
                        candidates.push(super::types::CompletionCandidate::new(
                            block.id().as_str().to_owned(),
                            super::types::CompletionCandidateKind::Block,
                            super::types::CompletionCandidateDetail::None,
                            replace_span.clone(),
                        ));
                    }
                }
            }
            super::types::CompletionContext::Speakers => {
                let Some(schema) = &self.schema else {
                    return QueryResult::Unavailable(QueryUnavailableReason::Incomplete(
                        QueryClass::Schema,
                    ));
                };
                for name in schema.speakers.keys() {
                    candidates.push(super::types::CompletionCandidate::new(
                        name.clone(),
                        super::types::CompletionCandidateKind::Speaker,
                        super::types::CompletionCandidateDetail::None,
                        replace_span.clone(),
                    ));
                }
            }
            super::types::CompletionContext::MetadataKeys => {
                let Some(schema) = &self.schema else {
                    return QueryResult::Unavailable(QueryUnavailableReason::Incomplete(
                        QueryClass::Schema,
                    ));
                };
                for name in schema.metadata.keys() {
                    candidates.push(super::types::CompletionCandidate::new(
                        name.clone(),
                        super::types::CompletionCandidateKind::MetadataKey,
                        super::types::CompletionCandidateDetail::None,
                        replace_span.clone(),
                    ));
                }
            }
            super::types::CompletionContext::MetadataValues { key: metadata_key } => {
                let Some(schema) = &self.schema else {
                    return QueryResult::Unavailable(QueryUnavailableReason::Incomplete(
                        QueryClass::Schema,
                    ));
                };
                let Some(definition) = schema.metadata.get(&metadata_key) else {
                    return QueryResult::NoMatch;
                };
                if let Some(recite_core::MetadataDomainDefinition::Flat(domain)) = definition
                    .domain
                    .as_ref()
                    .and_then(|name| schema.metadata_domains.get(name))
                {
                    for value in &domain.values {
                        candidates.push(super::types::CompletionCandidate::new(
                            value.clone(),
                            super::types::CompletionCandidateKind::MetadataValue,
                            super::types::CompletionCandidateDetail::None,
                            replace_span.clone(),
                        ));
                    }
                }
            }
            super::types::CompletionContext::Conditions => {
                let Some(schema) = &self.schema else {
                    return QueryResult::Unavailable(QueryUnavailableReason::Incomplete(
                        QueryClass::Schema,
                    ));
                };
                for (name, definition) in &schema.conditions {
                    candidates.push(super::types::CompletionCandidate::new(
                        name.clone(),
                        super::types::CompletionCandidateKind::Condition,
                        super::types::CompletionCandidateDetail::Parameters(
                            definition.params.len(),
                        ),
                        replace_span.clone(),
                    ));
                }
            }
            super::types::CompletionContext::Effects => {
                let Some(schema) = &self.schema else {
                    return QueryResult::Unavailable(QueryUnavailableReason::Incomplete(
                        QueryClass::Schema,
                    ));
                };
                for (name, definition) in &schema.effects {
                    candidates.push(super::types::CompletionCandidate::new(
                        name.clone(),
                        super::types::CompletionCandidateKind::Effect,
                        super::types::CompletionCandidateDetail::Parameters(
                            definition.params.len(),
                        ),
                        replace_span.clone(),
                    ));
                }
            }
            super::types::CompletionContext::AvailabilityReasons => {
                let Some(schema) = &self.schema else {
                    return QueryResult::Unavailable(QueryUnavailableReason::Incomplete(
                        QueryClass::Schema,
                    ));
                };
                for (name, definition) in &schema.availability_reasons {
                    candidates.push(super::types::CompletionCandidate::new(
                        name.as_str().to_owned(),
                        super::types::CompletionCandidateKind::AvailabilityReason,
                        super::types::CompletionCandidateDetail::Parameters(
                            definition.params.len(),
                        ),
                        replace_span.clone(),
                    ));
                }
            }
            super::types::CompletionContext::ProjectionQueries => {
                let Some(schema) = &self.schema else {
                    return QueryResult::Unavailable(QueryUnavailableReason::Incomplete(
                        QueryClass::Schema,
                    ));
                };
                for name in schema.projection_queries.keys() {
                    candidates.push(super::types::CompletionCandidate::new(
                        name.clone(),
                        super::types::CompletionCandidateKind::ProjectionQuery,
                        super::types::CompletionCandidateDetail::None,
                        replace_span.clone(),
                    ));
                }
            }
        }
        if candidates.is_empty() {
            QueryResult::NoMatch
        } else {
            QueryResult::Ready(candidates)
        }
    }
}
