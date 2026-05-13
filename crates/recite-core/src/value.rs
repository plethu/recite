use crate::SourceSpan;

/// Scalar metadata and schema values supported by the core model.
#[derive(Clone, Debug, PartialEq)]
pub enum ScalarValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

/// A metadata value. Arrays intentionally contain only scalar values.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Scalar(ScalarValue),
    Array(Vec<ScalarValue>),
}

impl From<ScalarValue> for Value {
    fn from(value: ScalarValue) -> Self {
        Self::Scalar(value)
    }
}

impl From<String> for ScalarValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for ScalarValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<i64> for ScalarValue {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<f64> for ScalarValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<bool> for ScalarValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

/// One metadata annotation, preserving its source location when available.
#[derive(Clone, Debug, PartialEq)]
pub struct MetadataEntry {
    pub key: String,
    pub value: Value,
    pub source_span: Option<SourceSpan>,
    pub key_span: Option<SourceSpan>,
    pub value_span: Option<SourceSpan>,
}

impl MetadataEntry {
    #[must_use]
    pub fn new(key: impl Into<String>, value: impl Into<Value>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            source_span: None,
            key_span: None,
            value_span: None,
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
}

/// Ordered metadata entries. Repeated keys are preserved.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Metadata {
    entries: Vec<MetadataEntry>,
}

impl Metadata {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn from_entries(entries: Vec<MetadataEntry>) -> Self {
        Self { entries }
    }

    pub fn push(&mut self, entry: MetadataEntry) {
        self.entries.push(entry);
    }

    pub fn iter(&self) -> impl Iterator<Item = &MetadataEntry> {
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
    pub fn as_slice(&self) -> &[MetadataEntry] {
        &self.entries
    }
}

impl IntoIterator for Metadata {
    type IntoIter = std::vec::IntoIter<MetadataEntry>;
    type Item = MetadataEntry;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<'a> IntoIterator for &'a Metadata {
    type IntoIter = std::slice::Iter<'a, MetadataEntry>;
    type Item = &'a MetadataEntry;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}
