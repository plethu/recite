use super::super::super::summary::MetadataValueKind;
use recite_core::{DocumentKey, SourceSpan};

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CompletionItem {
    pub(crate) identity: super::symbols::SymbolIdentity,
    pub(crate) kind: super::symbols::SymbolKind,
    pub(crate) declaration: super::symbols::SymbolLocation,
    pub(crate) replace_span: SourceSpan,
}
impl CompletionItem {
    #[must_use]
    pub fn identity(&self) -> &super::symbols::SymbolIdentity {
        &self.identity
    }
    #[must_use]
    pub const fn kind(&self) -> super::symbols::SymbolKind {
        self.kind
    }
    #[must_use]
    pub fn declaration(&self) -> &super::symbols::SymbolLocation {
        &self.declaration
    }
    #[must_use]
    pub fn replace_span(&self) -> &SourceSpan {
        &self.replace_span
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CompletionContext {
    Blocks { document: Option<DocumentKey> },
    Speakers,
    MetadataKeys,
    MetadataValues { key: String },
    Conditions,
    Effects,
    AvailabilityReasons,
    ProjectionQueries,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CompletionCandidate {
    name: String,
    kind: CompletionCandidateKind,
    detail: CompletionCandidateDetail,
    replace_span: SourceSpan,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CompletionCandidateKind {
    Block,
    Speaker,
    MetadataKey,
    MetadataValue,
    Condition,
    Effect,
    AvailabilityReason,
    ProjectionQuery,
}
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CompletionCandidateDetail {
    None,
    Type(MetadataValueKind),
    Parameters(usize),
}
impl CompletionCandidate {
    pub(crate) fn new(
        name: String,
        kind: CompletionCandidateKind,
        detail: CompletionCandidateDetail,
        replace_span: SourceSpan,
    ) -> Self {
        Self {
            name,
            kind,
            detail,
            replace_span,
        }
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub const fn kind(&self) -> CompletionCandidateKind {
        self.kind
    }
    #[must_use]
    pub fn detail(&self) -> &CompletionCandidateDetail {
        &self.detail
    }
    #[must_use]
    pub fn replace_span(&self) -> &SourceSpan {
        &self.replace_span
    }
}
