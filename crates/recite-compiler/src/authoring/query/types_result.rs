use super::super::super::summary::{FunctionReferenceKind, MetadataValue};
use super::completion::{CompletionCandidateDetail, CompletionCandidateKind};
use super::symbols::SymbolLocation;
use recite_core::{ProjectionOutputTarget, SchemaTypeRef};

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
    MissingMetadataContext,
    MalformedMetadataContext,
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
    /// A schema-owned symbol found in source prose.  The compiler resolves
    /// which schema namespace owns the token; hosts only localise and render
    /// the resulting fact.
    SchemaSymbol {
        name: String,
        kind: SemanticSymbolKind,
    },
    /// A schema-validated source metadata value, including the typed
    /// presentation context selected by the compiler.
    MetadataValueDetail {
        key: String,
        value: String,
        detail: MetadataValueDetail,
    },
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum MetadataValueDetail {
    Invalid,
    Speaker,
    Registry(SchemaTypeRef),
    Enum(SchemaTypeRef),
    Domain {
        name: String,
        context: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum SemanticSymbolKind {
    Speaker,
    Registry,
    MetadataDomain,
    Metadata,
    AvailabilityReason,
    Condition,
    Effect,
    ProjectionQuery,
    ProjectionProjector {
        inputs: usize,
        queries: usize,
        outputs: usize,
    },
    ProjectionOutput {
        projector: String,
        target: ProjectionOutputTarget,
        kind: String,
    },
    ProjectionLabel {
        arguments: usize,
    },
}
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct HoverInfo {
    pub(crate) location: SymbolLocation,
    pub(crate) facts: Vec<SemanticFact>,
    pub(crate) metadata_value: Option<(String, MetadataValueDetail)>,
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
    /// Returns the schema detail for a source metadata value, when available.
    #[must_use]
    pub fn metadata_value_detail(&self) -> Option<(&str, &MetadataValueDetail)> {
        self.metadata_value
            .as_ref()
            .map(|(value, detail)| (value.as_str(), detail))
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
