use recite_core::{BlockId, DocumentKey, SourceId, SourceSpan};

use super::super::summary::{FunctionReferenceKind, MetadataValue};

/// Whether a query has complete, recoverable, unavailable, or absent results.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum QueryResult<T> {
    Ready(T),
    Partial(T),
    Unavailable,
    NoMatch,
}

/// A compiler-owned symbol identity used by authoring queries.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum SymbolIdentity {
    Block(BlockId),
    Source(SourceId),
    MetadataKey(String),
    Function(String),
}

/// The structural kind of one symbol occurrence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SymbolKind {
    Block,
    BlockReference,
    StableId,
    Metadata,
    ConditionFunction,
    EffectFunction,
}

/// Whether an occurrence declares or uses a compiler-owned symbol.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SymbolRole {
    Definition,
    Reference,
    Annotation,
    Invocation,
}

/// One source-located symbol occurrence in a document.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SymbolLocation {
    pub(super) document: DocumentKey,
    pub(super) identity: SymbolIdentity,
    pub(super) kind: SymbolKind,
    pub(super) role: SymbolRole,
    pub(super) span: SourceSpan,
}

impl SymbolLocation {
    #[must_use]
    pub fn document(&self) -> &DocumentKey {
        &self.document
    }
    #[must_use]
    pub fn identity(&self) -> &SymbolIdentity {
        &self.identity
    }
    #[must_use]
    pub const fn kind(&self) -> SymbolKind {
        self.kind
    }
    #[must_use]
    pub const fn role(&self) -> SymbolRole {
        self.role
    }
    #[must_use]
    pub fn span(&self) -> &SourceSpan {
        &self.span
    }
}

/// Controls which symbol occurrences a symbol query returns.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SymbolQueryOptions {
    include_declarations: bool,
}

impl SymbolQueryOptions {
    #[must_use]
    pub const fn new(include_declarations: bool) -> Self {
        Self {
            include_declarations,
        }
    }

    #[must_use]
    pub const fn include_declarations(self) -> bool {
        self.include_declarations
    }
}

impl Default for SymbolQueryOptions {
    fn default() -> Self {
        Self::new(true)
    }
}

/// A typed completion candidate with a source replacement span.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CompletionItem {
    pub(super) identity: SymbolIdentity,
    pub(super) kind: SymbolKind,
    pub(super) declaration: SymbolLocation,
    pub(super) replace_span: SourceSpan,
}

impl CompletionItem {
    #[must_use]
    pub fn identity(&self) -> &SymbolIdentity {
        &self.identity
    }
    #[must_use]
    pub const fn kind(&self) -> SymbolKind {
        self.kind
    }
    #[must_use]
    pub fn declaration(&self) -> &SymbolLocation {
        &self.declaration
    }
    #[must_use]
    pub fn replace_span(&self) -> &SourceSpan {
        &self.replace_span
    }
}

/// Typed facts suitable for a host hover renderer.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum SemanticFact {
    Definition,
    Reference,
    MetadataValue(MetadataValue),
    Function {
        name: String,
        kind: FunctionReferenceKind,
        argument_count: usize,
    },
}

/// Structured hover data with no preformatted presentation text.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct HoverInfo {
    pub(super) location: SymbolLocation,
    pub(super) facts: Vec<SemanticFact>,
}

impl HoverInfo {
    #[must_use]
    pub fn location(&self) -> &SymbolLocation {
        &self.location
    }
    #[must_use]
    pub fn facts(&self) -> &[SemanticFact] {
        &self.facts
    }
}

/// Result of resolving a source symbol to declarations.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum NavigationResult {
    Unique(SymbolLocation),
    Missing,
    Ambiguous(Vec<SymbolLocation>),
}
