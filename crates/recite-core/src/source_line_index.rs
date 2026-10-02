use std::sync::Arc;

use crate::{SourcePosition, source_location::scalar_offset};

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
        let starts = std::iter::once(0)
            .chain(source.match_indices('\n').map(|(offset, _)| offset + 1))
            .collect();
        Self { source, starts }
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns a zero-based line, excluding LF but retaining any CR.
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
        let line = if index + 1 < self.starts.len() {
            line.strip_suffix('\r').unwrap_or(line)
        } else {
            line
        };
        scalar_offset(line, scalar).map(|offset| self.starts[index] + offset)
    }
}
