use super::super::super::summary::{FunctionReferenceKind, MetadataValue};
use super::completion::{CompletionCandidateDetail, CompletionCandidateKind};
use super::symbols::SymbolLocation;

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum QueryResult<T> {
    Ready(T),
    Partial {
        value: T,
        unavailable: Vec<QueryUnavailableReason>,
    },
    Unavailable(Vec<QueryUnavailableReason>),
    NoMatch,
}
impl<T> QueryResult<T> {
    pub(crate) fn partial(value: T, mut unavailable: Vec<QueryUnavailableReason>) -> Self {
        unavailable.sort();
        unavailable.dedup();
        Self::Partial { value, unavailable }
    }

    pub(crate) fn unavailable(reason: QueryUnavailableReason) -> Self {
        Self::Unavailable(vec![reason])
    }

    #[must_use]
    pub fn unavailable_reasons(&self) -> &[QueryUnavailableReason] {
        match self {
            Self::Partial { unavailable, .. } | Self::Unavailable(unavailable) => unavailable,
            Self::Ready(_) | Self::NoMatch => &[],
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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
