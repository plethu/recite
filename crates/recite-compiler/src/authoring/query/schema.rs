#[path = "schema/dialogue.rs"]
mod dialogue;
#[path = "schema/metadata.rs"]
mod metadata;
#[path = "schema/metadata_context.rs"]
mod metadata_context;
#[path = "schema/projection.rs"]
mod projection;

use recite_core::{
    DocumentKey, MetadataDomainDefinition, MetadataTarget, ProjectSchema, SourceSpan,
};

use super::super::snapshot::{AuthoringSnapshot, DocumentSnapshot};
use super::context::Site;
use super::types::{
    CompletionCandidate, CompletionCandidateKind, QueryClass, QueryResult, QueryUnavailableReason,
};

pub(super) fn complete_site(
    snapshot: &AuthoringSnapshot,
    key: &DocumentKey,
    document: &DocumentSnapshot,
    site: Site,
) -> QueryResult<Vec<CompletionCandidate>> {
    let (span, kind) = site_span_kind(&site);
    let mut candidates = Vec::new();
    let mut unavailable = Vec::new();
    if let Site::Blocks { target, .. } = site {
        dialogue::block_candidates(
            snapshot,
            key,
            target.as_ref(),
            document,
            &span,
            &mut candidates,
            &mut unavailable,
        );
        sort_candidates(&mut candidates);
        return result(candidates, unavailable);
    }
    let Some(schema) = &snapshot.schema else {
        return QueryResult::unavailable(QueryUnavailableReason::Incomplete(QueryClass::Schema));
    };
    match site {
        Site::Blocks { .. } => unreachable!("block sites return above"),
        Site::Speakers(_) => metadata::speaker_candidates(schema, kind, &span, &mut candidates),
        Site::MetadataKey { target, .. } => {
            metadata::key_candidates(schema, kind, target, &span, &mut candidates)
        }
        Site::MetadataValue {
            key: metadata_key,
            token,
            target,
            ..
        } => metadata::value_candidates(
            schema,
            document.source_text(),
            &metadata_key,
            target,
            &token,
            &mut unavailable,
            &mut candidates,
        ),
        Site::Conditions(_) => metadata::condition_candidates(schema, kind, &span, &mut candidates),
        Site::Effects(_) => metadata::effect_candidates(schema, kind, &span, &mut candidates),
        Site::AvailabilityReasons(_) => {
            metadata::reason_candidates(schema, kind, &span, &mut candidates)
        }
    }
    sort_candidates(&mut candidates);
    result(candidates, unavailable)
}

pub(super) fn contextual_metadata_context(
    schema: &ProjectSchema,
    text: &str,
    key: &str,
    line_number: u32,
    target: MetadataTarget,
) -> Option<String> {
    let domain_name = schema.metadata.get(key)?.domain.as_ref()?;
    let MetadataDomainDefinition::Contextual(domain) = schema.metadata_domains.get(domain_name)?
    else {
        return None;
    };
    match metadata_context::resolve_selector(text, &domain.selector, line_number, target) {
        metadata_context::SelectorResolution::Value(value) => Some(value.to_owned()),
        metadata_context::SelectorResolution::Missing
        | metadata_context::SelectorResolution::Malformed => None,
    }
}

fn sort_candidates(candidates: &mut [CompletionCandidate]) {
    candidates.sort_by(|left, right| {
        left.name()
            .cmp(right.name())
            .then_with(|| candidate_kind_rank(left.kind()).cmp(&candidate_kind_rank(right.kind())))
    });
}

fn result(
    candidates: Vec<CompletionCandidate>,
    unavailable: Vec<QueryUnavailableReason>,
) -> QueryResult<Vec<CompletionCandidate>> {
    if unavailable.is_empty() {
        QueryResult::Ready(candidates)
    } else {
        QueryResult::partial(candidates, unavailable)
    }
}

fn candidate_kind_rank(kind: CompletionCandidateKind) -> u8 {
    match kind {
        CompletionCandidateKind::Block => 0,
        CompletionCandidateKind::Speaker => 1,
        CompletionCandidateKind::MetadataKey => 2,
        CompletionCandidateKind::MetadataValue => 3,
        CompletionCandidateKind::Condition => 4,
        CompletionCandidateKind::Effect => 5,
        CompletionCandidateKind::AvailabilityReason => 6,
        CompletionCandidateKind::ProjectionQuery => 7,
        CompletionCandidateKind::ProjectionProjector => 8,
        CompletionCandidateKind::ProjectionInput => 9,
        CompletionCandidateKind::ProjectionQueryResult => 10,
        CompletionCandidateKind::ProjectionOutput => 11,
        CompletionCandidateKind::ProjectionLabel => 12,
    }
}

pub(super) fn explicit_projection_candidates(
    schema: &recite_core::ProjectSchema,
    projector: &str,
    span: &SourceSpan,
) -> Vec<CompletionCandidate> {
    let mut candidates = Vec::new();
    projection::candidates(schema, Some(projector), span, &mut candidates);
    candidates.sort_by(|left, right| {
        left.name()
            .cmp(right.name())
            .then_with(|| candidate_kind_rank(left.kind()).cmp(&candidate_kind_rank(right.kind())))
    });
    candidates
}

fn site_span_kind(site: &Site) -> (SourceSpan, CompletionCandidateKind) {
    match site {
        Site::Blocks { token, .. } => (token.clone(), CompletionCandidateKind::Block),
        Site::Speakers(span) => (span.clone(), CompletionCandidateKind::Speaker),
        Site::MetadataKey { span, .. } => (span.clone(), CompletionCandidateKind::MetadataKey),
        Site::MetadataValue { token, .. } => {
            (token.clone(), CompletionCandidateKind::MetadataValue)
        }
        Site::Conditions(span) => (span.clone(), CompletionCandidateKind::Condition),
        Site::Effects(span) => (span.clone(), CompletionCandidateKind::Effect),
        Site::AvailabilityReasons(span) => {
            (span.clone(), CompletionCandidateKind::AvailabilityReason)
        }
    }
}
