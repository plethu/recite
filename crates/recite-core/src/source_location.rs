use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use crate::CoreValueError;

/// A 1-based position in an author-visible source file.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct SourcePosition {
    line: NonZeroU32,
    column: NonZeroU32,
}

impl SourcePosition {
    pub const fn new(line: u32, column: u32) -> Result<Self, CoreValueError> {
        let Some(line) = NonZeroU32::new(line) else {
            return Err(CoreValueError::ZeroSourceLine);
        };
        let Some(column) = NonZeroU32::new(column) else {
            return Err(CoreValueError::ZeroSourceColumn);
        };

        Ok(Self { line, column })
    }

    #[must_use]
    pub const fn line(self) -> u32 {
        self.line.get()
    }

    #[must_use]
    pub const fn column(self) -> u32 {
        self.column.get()
    }
}

/// A span in a source file, suitable for diagnostics and editor surfaces.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SourceSpan {
    pub file: String,
    pub start: SourcePosition,
    pub end: Option<SourcePosition>,
}

impl SourceSpan {
    #[must_use]
    pub fn new(
        file: impl Into<String>,
        start: SourcePosition,
        end: Option<SourcePosition>,
    ) -> Self {
        Self {
            file: file.into(),
            start,
            end,
        }
    }

    #[must_use]
    pub fn point(file: impl Into<String>, position: SourcePosition) -> Self {
        Self::new(file, position, None)
    }
}

/// Return the byte boundary for a one-based line and Unicode scalar column.
/// The end of a line is valid for an exclusive edit range; CRLF bytes are
/// outside that line's editable columns.
#[must_use]
pub fn byte_offset_for_position(source: &str, position: SourcePosition) -> Option<usize> {
    let wanted_line = position.line();
    let wanted_scalar = usize::try_from(position.column().checked_sub(1)?).ok()?;
    let bytes = source.as_bytes();
    let mut line_start = 0;
    let mut line = 1;

    for (index, byte) in bytes.iter().copied().enumerate() {
        if byte != b'\n' {
            continue;
        }
        let line_end =
            index.saturating_sub(usize::from(index > line_start && bytes[index - 1] == b'\r'));
        if line == wanted_line {
            return scalar_offset(&source[line_start..line_end], wanted_scalar)
                .map(|offset| line_start + offset);
        }
        line_start = index + 1;
        line = line.saturating_add(1);
    }

    (line == wanted_line)
        .then(|| {
            scalar_offset(&source[line_start..], wanted_scalar).map(|offset| line_start + offset)
        })
        .flatten()
}

fn scalar_offset(line: &str, scalar: usize) -> Option<usize> {
    line.char_indices()
        .nth(scalar)
        .map(|(offset, _)| offset)
        .or_else(|| (line.chars().count() == scalar).then_some(line.len()))
}

pub(crate) fn source_position(line: usize, column: usize) -> Option<SourcePosition> {
    let line = u32::try_from(line).ok()?;
    let column = u32::try_from(column).ok()?;
    SourcePosition::new(line, column).ok()
}

// Invariant: computed source positions start at 1:1 and only increase from there.
#[allow(
    clippy::expect_used,
    reason = "byte offsets are converted from the canonical 1-based source origin"
)]
pub(crate) fn position_for_byte_offset(source: &str, offset: usize) -> SourcePosition {
    let mut line = 1usize;
    let mut column = 1usize;
    for (index, character) in source.char_indices() {
        if index >= offset {
            break;
        }
        if character == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    source_position(line, column).expect("line and column start at one")
}

// Invariant: 1:1 is the canonical valid source start position.
#[allow(
    clippy::expect_used,
    reason = "this helper owns the canonical valid 1:1 source position"
)]
pub(crate) fn point_one() -> SourcePosition {
    SourcePosition::new(1, 1).expect("1-based position is valid")
}

#[cfg(test)]
mod tests;
