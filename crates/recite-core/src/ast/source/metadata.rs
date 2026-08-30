use crate::SourceSpan;

/// Scalar metadata values as authored in source.
#[derive(Clone, Debug, PartialEq)]
pub enum SourceMetadataScalar {
    Symbol(String),
    StringLiteral(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
}

/// Source metadata values preserve scalar kind before schema validation.
#[derive(Clone, Debug, PartialEq)]
pub enum SourceMetadataValue {
    Scalar(SourceMetadataScalar),
    Array(Vec<SourceMetadataScalar>),
}

impl From<SourceMetadataScalar> for SourceMetadataValue {
    fn from(value: SourceMetadataScalar) -> Self {
        Self::Scalar(value)
    }
}

/// One source metadata annotation, preserving spans when available.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SourceMetadataEntry {
    pub key: String,
    pub value: SourceMetadataValue,
    pub source_span: Option<SourceSpan>,
    pub key_span: Option<SourceSpan>,
    pub value_span: Option<SourceSpan>,
    value_element_spans: Vec<SourceSpan>,
}

impl SourceMetadataEntry {
    #[must_use]
    pub fn new(key: impl Into<String>, value: impl Into<SourceMetadataValue>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            source_span: None,
            key_span: None,
            value_span: None,
            value_element_spans: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_source_span(mut self, source_span: SourceSpan) -> Self {
        self.source_span = Some(source_span);
        self
    }

    #[must_use]
    pub fn with_key_value_spans(
        mut self,
        key_span: SourceSpan,
        value_span: Option<SourceSpan>,
    ) -> Self {
        self.key_span = Some(key_span);
        self.value_span = value_span;
        self
    }

    #[must_use]
    pub fn with_value_element_spans(mut self, spans: Vec<SourceSpan>) -> Self {
        self.value_element_spans = spans;
        self
    }

    #[must_use]
    pub fn value_element_spans(&self) -> &[SourceSpan] {
        &self.value_element_spans
    }
}

/// Ordered source metadata entries. Repeated keys are preserved.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceMetadata {
    entries: Vec<SourceMetadataEntry>,
}

impl SourceMetadata {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn from_entries(entries: Vec<SourceMetadataEntry>) -> Self {
        Self { entries }
    }

    pub fn push(&mut self, entry: SourceMetadataEntry) {
        self.entries.push(entry);
    }

    pub fn iter(&self) -> impl Iterator<Item = &SourceMetadataEntry> {
        self.entries.iter()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn as_slice(&self) -> &[SourceMetadataEntry] {
        &self.entries
    }
}

impl IntoIterator for SourceMetadata {
    type IntoIter = std::vec::IntoIter<SourceMetadataEntry>;
    type Item = SourceMetadataEntry;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<'a> IntoIterator for &'a SourceMetadata {
    type IntoIter = std::slice::Iter<'a, SourceMetadataEntry>;
    type Item = &'a SourceMetadataEntry;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}
