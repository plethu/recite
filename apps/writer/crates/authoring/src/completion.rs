//! Completion queries use the current draft without applying it to the document.
use crate::{Document, EditError};
use recite_compiler::{CompletionCandidate, QueryResult};
use recite_core::SourcePosition;

#[derive(Clone, Debug, PartialEq)]
pub struct SourceCompletions {
    source: String,
    candidates: Vec<CompletionCandidate>,
}
impl SourceCompletions {
    pub fn candidates(&self) -> &[CompletionCandidate] {
        &self.candidates
    }

    /// Byte range and replacement belong to exactly the draft that was queried.
    pub fn replacement(
        &self,
        current: &str,
        index: usize,
    ) -> Result<(std::ops::Range<usize>, &str), EditError> {
        if current != self.source {
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
        let overlay = Self::in_project(self.key().clone(), draft, self.project_context())?;
        let candidates = match overlay.kernel().snapshot().complete(self.key(), position) {
            QueryResult::Ready(value) | QueryResult::Partial { value, .. } => value,
            _ => Vec::new(),
        };
        Ok(SourceCompletions {
            source: draft.into(),
            candidates,
        })
    }
}
