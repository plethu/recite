use recite_core::{BlockId, SourceId, SourceMetadataScalar, SourceMetadataValue, SourceSpan};

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BlockDefinitionSummary {
    pub(super) id: BlockId,
    pub(super) id_span: Option<SourceSpan>,
    pub(super) span: SourceSpan,
}

impl BlockDefinitionSummary {
    #[must_use]
    pub fn id(&self) -> &BlockId {
        &self.id
    }
    #[must_use]
    pub fn span(&self) -> &SourceSpan {
        &self.span
    }
    #[must_use]
    pub fn id_span(&self) -> Option<&SourceSpan> {
        self.id_span.as_ref()
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BlockReferenceSummary {
    pub(super) file: Option<String>,
    pub(super) file_span: Option<SourceSpan>,
    pub(super) block_id: BlockId,
    pub(super) block_id_span: Option<SourceSpan>,
    pub(super) span: SourceSpan,
}

impl BlockReferenceSummary {
    #[must_use]
    pub fn file(&self) -> Option<&str> {
        self.file.as_deref()
    }
    #[must_use]
    pub fn block_id(&self) -> &BlockId {
        &self.block_id
    }
    #[must_use]
    pub fn file_span(&self) -> Option<&SourceSpan> {
        self.file_span.as_ref()
    }
    #[must_use]
    pub fn block_id_span(&self) -> Option<&SourceSpan> {
        self.block_id_span.as_ref()
    }
    #[must_use]
    pub fn span(&self) -> &SourceSpan {
        &self.span
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StableIdKind {
    Line,
    Choice,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StableIdSummary {
    pub(super) kind: StableIdKind,
    pub(super) source_id: SourceId,
    pub(super) source_id_span: Option<SourceSpan>,
    pub(super) insertion_span: Option<SourceSpan>,
    pub(super) span: SourceSpan,
}

impl StableIdSummary {
    #[must_use]
    pub const fn kind(&self) -> StableIdKind {
        self.kind
    }
    #[must_use]
    pub fn source_id(&self) -> &SourceId {
        &self.source_id
    }
    #[must_use]
    pub fn source_id_span(&self) -> Option<&SourceSpan> {
        self.source_id_span.as_ref()
    }
    #[must_use]
    pub fn insertion_span(&self) -> Option<&SourceSpan> {
        self.insertion_span.as_ref()
    }
    #[must_use]
    pub fn span(&self) -> &SourceSpan {
        &self.span
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct MetadataSummary {
    pub(super) key: String,
    pub(super) source_span: Option<SourceSpan>,
    pub(super) key_span: Option<SourceSpan>,
    pub(super) value_span: Option<SourceSpan>,
    pub(super) value: MetadataValue,
}

impl MetadataSummary {
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }
    #[must_use]
    pub fn key_span(&self) -> Option<&SourceSpan> {
        self.key_span.as_ref()
    }
    #[must_use]
    pub fn source_span(&self) -> Option<&SourceSpan> {
        self.source_span.as_ref()
    }
    #[must_use]
    pub fn value_span(&self) -> Option<&SourceSpan> {
        self.value_span.as_ref()
    }
    #[must_use]
    pub fn value(&self) -> &MetadataValue {
        &self.value
    }
}

/// A typed metadata value preserved from source before schema validation.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum MetadataValue {
    Scalar(MetadataScalar),
    Array(Vec<MetadataScalar>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum MetadataValueKind {
    Symbol,
    String,
    Integer,
    Float,
    Boolean,
    Array,
}

impl From<&SourceMetadataValue> for MetadataValue {
    fn from(value: &SourceMetadataValue) -> Self {
        match value {
            SourceMetadataValue::Scalar(scalar) => Self::Scalar(scalar.into()),
            SourceMetadataValue::Array(values) => {
                Self::Array(values.iter().map(Into::into).collect())
            }
        }
    }
}

/// A scalar metadata value with source-declared type preserved.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum MetadataScalar {
    Symbol(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl From<&SourceMetadataScalar> for MetadataScalar {
    fn from(value: &SourceMetadataScalar) -> Self {
        match value {
            SourceMetadataScalar::Symbol(value) => Self::Symbol(value.clone()),
            SourceMetadataScalar::StringLiteral(value) => Self::String(value.clone()),
            SourceMetadataScalar::Integer(value) => Self::Integer(*value),
            SourceMetadataScalar::Float(value) => Self::Float(*value),
            SourceMetadataScalar::Bool(value) => Self::Boolean(*value),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FunctionReferenceKind {
    BooleanCondition,
    MatchCondition,
    DeferredEffect,
    ImmediateEffect,
    BlockingEffect,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct FunctionReferenceSummary {
    pub(super) name: String,
    pub(super) span: SourceSpan,
    pub(super) argument_count: usize,
    pub(super) kind: FunctionReferenceKind,
}

impl FunctionReferenceSummary {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub fn span(&self) -> &SourceSpan {
        &self.span
    }
    #[must_use]
    pub const fn argument_count(&self) -> usize {
        self.argument_count
    }
    #[must_use]
    pub const fn kind(&self) -> FunctionReferenceKind {
        self.kind
    }
}
