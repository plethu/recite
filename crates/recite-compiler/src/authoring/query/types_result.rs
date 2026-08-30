use super::super::super::summary::{FunctionReferenceKind, MetadataValue};
use super::completion::{CompletionCandidateDetail, CompletionCandidateKind};
use super::symbols::SymbolLocation;
use recite_core::{DocumentKey, ProjectionOutputTarget, SchemaTypeRef, SourceSpan};

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
    /// A source clause marker whose syntax and range were parsed by the
    /// compiler.  Hosts only choose the localised explanation.
    Clause {
        kind: ClauseKind,
    },
    /// A schema-backed availability reason assignment parsed from source.
    AvailabilityReason {
        name: String,
        template: String,
        parameters: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ClauseKind {
    Requires,
    If,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CompletionSiteKind {
    Block,
    Speaker,
    MetadataKey,
    MetadataValue,
    Condition,
    Effect,
    AvailabilityReason,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CompletionSite {
    kind: CompletionSiteKind,
    span: SourceSpan,
    block_target: Option<DocumentKey>,
}

impl CompletionSite {
    pub(crate) fn new(
        kind: CompletionSiteKind,
        span: SourceSpan,
        block_target: Option<DocumentKey>,
    ) -> Self {
        Self {
            kind,
            span,
            block_target,
        }
    }

    #[must_use]
    pub const fn kind(&self) -> CompletionSiteKind {
        self.kind
    }

    #[must_use]
    pub fn span(&self) -> &SourceSpan {
        &self.span
    }

    #[must_use]
    pub fn block_target(&self) -> Option<&DocumentKey> {
        self.block_target.as_ref()
    }
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
