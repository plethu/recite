use super::super::super::summary::{FunctionReferenceKind, MetadataValue};
use super::completion::{CompletionCandidateDetail, CompletionCandidateKind};
use super::symbols::SymbolLocation;

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum QueryResult<T> {
    Ready(T),
    Partial(T),
    Unavailable(QueryUnavailableReason),
    NoMatch,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum QueryClass {
    Diagnostics,
    BlockDefinitions,
    BlockReferences,
    StableIds,
    Metadata,
    ConditionFunctions,
    EffectFunctions,
    Schema,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum QueryUnavailableReason {
    Incomplete(QueryClass),
    Unsupported,
}
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
    SchemaCandidate {
        name: String,
        kind: CompletionCandidateKind,
        detail: CompletionCandidateDetail,
    },
}
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct HoverInfo {
    pub(crate) location: SymbolLocation,
    pub(crate) facts: Vec<SemanticFact>,
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
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum NavigationResult {
    Unique(SymbolLocation),
    Missing,
    Ambiguous(Vec<SymbolLocation>),
    Unsupported,
}
