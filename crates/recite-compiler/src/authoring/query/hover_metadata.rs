use recite_core::{DocumentKey, SourcePosition};

use super::super::snapshot::{AuthoringSnapshot, DocumentSnapshot};
use super::super::summary::{MetadataScalar, MetadataSummary, MetadataValue};
use super::context;
use super::types::{
    CompletionCandidate, CompletionCandidateDetail, MetadataValueDetail, QueryResult, SemanticFact,
};

impl AuthoringSnapshot {
    pub(super) fn metadata_value_detail(
        &self,
        key: &DocumentKey,
        document: &DocumentSnapshot,
        position: SourcePosition,
        metadata: &MetadataSummary,
    ) -> Option<(String, MetadataValueDetail)> {
        let (metadata_key, target) = match context::at(key, document.source_text(), position)? {
            context::Site::MetadataValue {
                key: metadata_key,
                target,
                ..
            } => (metadata_key, target),
            context::Site::Speakers(_) => ("speaker".to_owned(), recite_core::MetadataTarget::Line),
            _ => return None,
        };
        if metadata_key != metadata.key() {
            return None;
        }
        let (value, _) = context::token_at(key, document.source_text(), position)?;
        let completion = self.complete(key, position);
        let candidate = match completion {
            QueryResult::Ready(candidates)
            | QueryResult::Partial {
                value: candidates, ..
            } => candidates
                .into_iter()
                .find(|candidate| candidate.name() == value),
            QueryResult::Unavailable(_) | QueryResult::NoMatch => None,
        };
        let detail = match (
            metadata_contains_symbol(metadata.value(), &value),
            candidate,
        ) {
            (false, _) | (true, None) => MetadataValueDetail::Invalid,
            (true, Some(candidate)) => match candidate.detail() {
                CompletionCandidateDetail::Speaker { .. } => MetadataValueDetail::Speaker,
                CompletionCandidateDetail::Metadata {
                    domain: Some(domain),
                    ..
                } => MetadataValueDetail::Domain {
                    name: domain.clone(),
                    context: self.schema.as_deref().and_then(|schema| {
                        super::schema::contextual_metadata_context(
                            schema,
                            document.source_text(),
                            metadata.key(),
                            position.line(),
                            target,
                        )
                    }),
                },
                CompletionCandidateDetail::SchemaType(
                    type_ref @ recite_core::SchemaTypeRef::Registry(_),
                ) => MetadataValueDetail::Registry(type_ref.clone()),
                CompletionCandidateDetail::SchemaType(
                    type_ref @ recite_core::SchemaTypeRef::Enum(_),
                ) => MetadataValueDetail::Enum(type_ref.clone()),
                _ => return None,
            },
        };
        Some((value, detail))
    }

    pub(super) fn metadata_candidate_fact(
        &self,
        key: &DocumentKey,
        text: &str,
        position: SourcePosition,
        candidate: &CompletionCandidate,
    ) -> Option<SemanticFact> {
        let context::Site::MetadataValue {
            key: metadata_key,
            value: raw_value,
            target,
            ..
        } = context::at(key, text, position)?
        else {
            return None;
        };
        let parsed_value = recite_parser::parse_metadata_value(&raw_value)?;
        let parsed_symbol = match parsed_value {
            recite_core::SourceMetadataValue::Scalar(
                recite_core::SourceMetadataScalar::Symbol(value),
            ) if value == candidate.name() => value,
            recite_core::SourceMetadataValue::Array(values)
                if values.iter().any(|value| {
                    matches!(value, recite_core::SourceMetadataScalar::Symbol(value) if value == candidate.name())
                }) => candidate.name().to_owned(),
            _ => return None,
        };
        let detail = match candidate.detail() {
            CompletionCandidateDetail::Speaker { .. } => MetadataValueDetail::Speaker,
            CompletionCandidateDetail::Metadata {
                domain: Some(domain),
                ..
            } => MetadataValueDetail::Domain {
                name: domain.clone(),
                context: self.schema.as_deref().and_then(|schema| {
                    super::schema::contextual_metadata_context(
                        schema,
                        text,
                        &metadata_key,
                        position.line(),
                        target,
                    )
                }),
            },
            CompletionCandidateDetail::SchemaType(
                type_ref @ recite_core::SchemaTypeRef::Registry(_),
            ) => MetadataValueDetail::Registry(type_ref.clone()),
            CompletionCandidateDetail::SchemaType(
                type_ref @ recite_core::SchemaTypeRef::Enum(_),
            ) => MetadataValueDetail::Enum(type_ref.clone()),
            _ => return None,
        };
        Some(SemanticFact::MetadataValueDetail {
            key: metadata_key,
            value: parsed_symbol,
            detail,
        })
    }
}

fn metadata_contains_symbol(value: &MetadataValue, expected: &str) -> bool {
    match value {
        MetadataValue::Scalar(MetadataScalar::Symbol(value)) => value == expected,
        MetadataValue::Array(values) => values
            .iter()
            .any(|value| matches!(value, MetadataScalar::Symbol(value) if value == expected)),
        _ => false,
    }
}
