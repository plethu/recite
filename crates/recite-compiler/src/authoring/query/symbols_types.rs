use recite_core::{DocumentKey, SourceId, SourceSpan};

use super::result::ClauseKind;
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum SymbolIdentity {
    Block(recite_core::BlockId),
    Source(SourceId),
    MetadataKey(String),
    Function(String),
    Schema(String),
    Clause(ClauseKind),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SymbolKind {
    Block,
    BlockReference,
    StableId,
    Metadata,
    ConditionFunction,
    EffectFunction,
    Schema,
    Clause,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SymbolRole {
    Definition,
    Reference,
    Annotation,
    Invocation,
}
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SymbolLocation {
    pub(crate) document: DocumentKey,
    pub(crate) identity: SymbolIdentity,
    pub(crate) kind: SymbolKind,
    pub(crate) role: SymbolRole,
    pub(crate) span: SourceSpan,
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
