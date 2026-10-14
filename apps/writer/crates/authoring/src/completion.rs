//! Completion queries use the current draft without applying it to the document.
use crate::{Document, EditError};
use recite_compiler::authoring::{
    AuthoringRequest, CancellationToken, CompletionCandidate, DocumentVersion, OpenDocument,
    QueryResult,
};
use recite_core::SourcePosition;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct SourceCompletions {
    source: Arc<str>,
    candidates: Vec<CompletionCandidate>,
}
impl SourceCompletions {
    /// Exact draft bytes captured with these replacement spans.
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn candidates(&self) -> &[CompletionCandidate] {
        &self.candidates
    }

    /// Byte range and replacement belong to exactly the draft that was queried.
    pub fn replacement(
        &self,
        current: &str,
        index: usize,
    ) -> Result<(std::ops::Range<usize>, &str), EditError> {
        if current != self.source.as_ref() {
            return Err(EditError::Stale);
        }
        let candidate = self.candidates.get(index).ok_or(EditError::Position)?;
        let span = candidate.replace_span();
        let start = crate::projection::offset(current, span.start)?;
        let end = if let Some(position) = span.end {
            let offset = crate::projection::offset(current, position)?;
            offset
                + current[offset..]
                    .chars()
                    .next()
                    .filter(|c| !matches!(c, '\r' | '\n'))
                    .map_or(0, char::len_utf8)
        } else {
            start
        };
        Ok((start..end, candidate.name()))
    }
}
impl Document {
    pub fn complete_draft(
        &self,
        draft: &str,
        position: SourcePosition,
    ) -> Result<SourceCompletions, EditError> {
        let accepted = draft == self.source();
        let source = if accepted {
            self.source_snapshot()
        } else {
            Arc::from(draft)
        };
        let updated;
        let fallback;
        let snapshot = if accepted {
            self.kernel().snapshot()
        } else if let Some(version) = self.revision().checked_add(1) {
            updated = self.kernel().updated(
                AuthoringRequest::new(
                    self.kernel().snapshot().generation(),
                    self.saved_documents().to_vec(),
                    [OpenDocument::from_shared(
                        self.key().clone(),
                        DocumentVersion::new(version),
                        source.clone(),
                    )],
                ),
                &CancellationToken::new(),
            )?;
            updated.snapshot()
        } else {
            // A read-only draft query still works after the accepted version is exhausted.
            fallback = Self::in_project(self.key().clone(), draft, self.project_context())?;
            fallback.kernel().snapshot()
        };
        let candidates = match snapshot.complete(self.key(), position) {
            QueryResult::Ready(value) | QueryResult::Partial { value, .. } => value,
            _ => Vec::new(),
        };
        Ok(SourceCompletions { source, candidates })
    }
}
