//! Completion queries use the current draft without applying it to the document.
use crate::{Document, EditError};
use recite_compiler::authoring::{CancellationToken, CompletionCandidate};
use recite_core::{SourceLineIndex, SourcePosition};

#[derive(Clone, PartialEq)]
pub struct SourceCompletions {
    pub(super) source: SourceLineIndex,
    pub(super) candidates: Vec<CompletionCandidate>,
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
        self.completion_snapshot()
            .complete(draft, position, &CancellationToken::new())
    }
}
