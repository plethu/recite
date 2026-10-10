use std::sync::Arc;

use crate::{SourcePosition, source_lines, source_location::scalar_offset};

/// Line boundaries bound to immutable source bytes. Clones share both.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLineIndex {
    source: Arc<str>,
    starts: Arc<[usize]>,
}

impl SourceLineIndex {
    #[must_use]
    pub fn new(source: impl Into<Arc<str>>) -> Self {
        let source = source.into();
        let starts = source_lines(&source)
            .scan(0, |offset, (content, terminator)| {
                let start = *offset;
                *offset += content.len() + terminator.len();
                Some(start)
            })
            .collect();
        Self { source, starts }
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns a zero-based line, excluding the final terminator byte.
    /// A CRLF line retains its CR for compatibility with source-backed slices.
    #[must_use]
    pub fn line(&self, index: usize) -> Option<&str> {
        let start = *self.starts.get(index)?;
        let end = self
            .starts
            .get(index + 1)
            .map_or(self.source.len(), |next| next - 1);
        self.source.get(start..end)
    }

    /// Exact byte boundary for a one-based Unicode scalar position.
    /// CRLF is outside editable columns, matching `byte_offset_for_position`.
    #[must_use]
    pub fn byte_offset(&self, position: SourcePosition) -> Option<usize> {
        let index = usize::try_from(position.line().checked_sub(1)?).ok()?;
        let scalar = usize::try_from(position.column().checked_sub(1)?).ok()?;
        let line = self.line(index)?;
        // A final logical line has no terminator, so trimming is also safe there.
        let line = line.strip_suffix('\r').unwrap_or(line);
        scalar_offset(line, scalar).map(|offset| self.starts[index] + offset)
    }
}
