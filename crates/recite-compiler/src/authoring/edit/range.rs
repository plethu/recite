use recite_core::{DocumentKey, SourcePosition};

/// A source range whose end is exclusive.
///
/// `SourceSpan` remains the diagnostic/source-model type and has an inclusive
/// end. This type is deliberately separate so callers can apply an edit
/// without having to infer whether the final source character is included.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceRange {
    start: SourcePosition,
    end: SourcePosition,
}

impl SourceRange {
    /// Creates a start-inclusive, end-exclusive source range.
    #[must_use]
    pub const fn new(start: SourcePosition, end: SourcePosition) -> Self {
        Self { start, end }
    }

    /// Creates an empty range at one source position.
    #[must_use]
    pub const fn point(position: SourcePosition) -> Self {
        Self::new(position, position)
    }

    #[must_use]
    pub const fn start(self) -> SourcePosition {
        self.start
    }

    #[must_use]
    pub const fn end(self) -> SourcePosition {
        self.end
    }
}

/// A host-neutral replacement over one logical document.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SourceEdit {
    document: DocumentKey,
    range: SourceRange,
    replacement: String,
}

impl SourceEdit {
    /// Creates a source replacement. The caller applies it to its own
    /// document store after checking the plan preconditions.
    #[must_use]
    pub fn new(document: DocumentKey, range: SourceRange, replacement: impl Into<String>) -> Self {
        Self {
            document,
            range,
            replacement: replacement.into(),
        }
    }

    #[must_use]
    pub fn document(&self) -> &DocumentKey {
        &self.document
    }

    #[must_use]
    pub const fn range(&self) -> SourceRange {
        self.range
    }

    #[must_use]
    pub fn replacement(&self) -> &str {
        &self.replacement
    }
}
