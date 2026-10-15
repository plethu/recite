//! Completion queries use the current draft without applying it to the document.
use crate::{Document, EditError};
use recite_compiler::authoring::{
    AuthoringRequest, CancellationToken, CompletionCandidate, DocumentVersion, OpenDocument,
    QueryResult,
};
use recite_core::{SourceLineIndex, SourcePosition};
use std::sync::Arc;

#[derive(Clone, PartialEq)]
pub struct SourceCompletions {
    source: SourceLineIndex,
    candidates: Vec<CompletionCandidate>,
}
impl SourceCompletions {
    /// Exact draft bytes captured with these replacement spans.
    pub fn source(&self) -> &str {
        self.source.source()
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
        if !std::ptr::eq(current, self.source()) && current != self.source() {
            return Err(EditError::Stale);
        }
        let candidate = self.candidates.get(index).ok_or(EditError::Position)?;
        let span = candidate.replace_span();
        let start = self
            .source
            .byte_offset(span.start)
            .ok_or(EditError::Position)?;
        let end = if let Some(position) = span.end {
            let offset = self
                .source
                .byte_offset(position)
                .ok_or(EditError::Position)?;
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

impl std::fmt::Debug for SourceCompletions {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SourceCompletions")
            .field("source", &self.source())
            .field("candidates", &self.candidates)
            .finish()
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
        let source = snapshot
            .document(self.key())
            .ok_or(EditError::Position)?
            .source_index()
            .clone();
        Ok(SourceCompletions { source, candidates })
    }
}
