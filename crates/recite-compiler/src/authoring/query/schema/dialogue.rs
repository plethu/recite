use recite_core::{DocumentKey, SourceSpan};

use super::super::super::snapshot::{AuthoringSnapshot, DocumentSnapshot};
use super::super::types::{
    CompletionCandidate, CompletionCandidateDetail, CompletionCandidateKind, QueryClass,
    QueryUnavailableReason,
};

pub(super) fn block_candidates(
    snapshot: &AuthoringSnapshot,
    _key: &DocumentKey,
    target: Option<&DocumentKey>,
    document: &DocumentSnapshot,
    span: &SourceSpan,
    candidates: &mut Vec<CompletionCandidate>,
    unavailable: &mut Vec<QueryUnavailableReason>,
) {
    if !document.participation().block_references().is_complete() {
        unavailable.push(QueryUnavailableReason::Incomplete(
            QueryClass::BlockReferences,
        ));
    }
    for target_document in snapshot.documents() {
        if target.is_some_and(|target| target != target_document.key()) {
            continue;
        }
        if !target_document
            .participation()
            .block_definitions()
            .is_complete()
        {
            unavailable.push(QueryUnavailableReason::Incomplete(
                QueryClass::BlockDefinitions,
            ));
            continue;
        }
        candidates.extend(target_document.summary().blocks().iter().map(|block| {
            CompletionCandidate::new(
                block.id().as_str().to_owned(),
                CompletionCandidateKind::Block,
                CompletionCandidateDetail::None,
                span.clone(),
            )
        }));
    }
}
