use std::num::NonZeroU32;

use crate::CoreValueError;

/// A 1-based position in an author-visible source file.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
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
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
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

pub(crate) fn source_position(line: usize, column: usize) -> Option<SourcePosition> {
    let line = u32::try_from(line).ok()?;
    let column = u32::try_from(column).ok()?;
    SourcePosition::new(line, column).ok()
}

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

pub(crate) fn point_one() -> SourcePosition {
    SourcePosition::new(1, 1).expect("1-based position is valid")
}
