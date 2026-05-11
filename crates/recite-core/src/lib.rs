//! Core Recite AST, identifiers, values, diagnostics, and schema model.

#![forbid(unsafe_code)]

use std::fmt;

/// A 1-based position in an author-visible source file.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct SourcePosition {
    pub line: u32,
    pub column: u32,
}

impl SourcePosition {
    #[must_use]
    pub const fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

/// A span in a source file, suitable for diagnostics and editor surfaces.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SourceSpan {
    pub file: String,
    pub start: SourcePosition,
    pub end: Option<SourcePosition>,
}

impl SourceSpan {
    #[must_use]
    pub fn new(
        file: impl Into<String>,
        start: SourcePosition,
        end: Option<SourcePosition>,
    ) -> Self {
        Self {
            file: file.into(),
            start,
            end,
        }
    }

    #[must_use]
    pub fn point(file: impl Into<String>, position: SourcePosition) -> Self {
        Self::new(file, position, None)
    }
}

/// Stable diagnostic severity shared by compiler, CLI, and LSP surfaces.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// A stable diagnostic code, for example `RECITE_PARSE001`.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DiagnosticCode(String);

impl DiagnosticCode {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Additional source location related to a primary diagnostic.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RelatedSpan {
    pub span: SourceSpan,
    pub message: String,
}

impl RelatedSpan {
    #[must_use]
    pub fn new(span: SourceSpan, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

/// A structured diagnostic that can be rendered by CLI and editor tooling.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub span: SourceSpan,
    pub related: Vec<RelatedSpan>,
    pub help: Option<String>,
}

impl Diagnostic {
    #[must_use]
    pub fn new(
        code: impl Into<DiagnosticCode>,
        severity: DiagnosticSeverity,
        message: impl Into<String>,
        span: SourceSpan,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            span,
            related: Vec::new(),
            help: None,
        }
    }

    #[must_use]
    pub fn with_related(mut self, related: impl IntoIterator<Item = RelatedSpan>) -> Self {
        self.related.extend(related);
        self
    }

    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

impl From<&str> for DiagnosticCode {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for DiagnosticCode {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

define_id!(LineId);
define_id!(ChoiceId);
define_id!(BlockId);
define_id!(EffectId);
define_id!(SpeakerId);

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
}

impl MetadataEntry {
    #[must_use]
    pub fn new(key: impl Into<String>, value: impl Into<Value>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            source_span: None,
        }
    }

    #[must_use]
    pub fn with_source_span(mut self, source_span: SourceSpan) -> Self {
        self.source_span = Some(source_span);
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

    #[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_spans_support_points_and_ranges() {
        let start = SourcePosition::new(3, 5);
        let end = SourcePosition::new(3, 12);

        let point = SourceSpan::point("dialogue/tavern.recite", start);
        assert_eq!(point.file, "dialogue/tavern.recite");
        assert_eq!(point.start, start);
        assert_eq!(point.end, None);

        let range = SourceSpan::new("dialogue/tavern.recite", start, Some(end));
        assert_eq!(range.end, Some(end));
    }

    #[test]
    fn diagnostics_keep_stable_structured_fields() {
        let primary = SourceSpan::new(
            "dialogue/tavern.recite",
            SourcePosition::new(8, 1),
            Some(SourcePosition::new(8, 14)),
        );
        let related = RelatedSpan::new(
            SourceSpan::point("dialogue/tavern.recite", SourcePosition::new(2, 4)),
            "first declaration is here",
        );

        let diagnostic = Diagnostic::new(
            "RECITE_ID001",
            DiagnosticSeverity::Error,
            "duplicate line ID",
            primary.clone(),
        )
        .with_related([related.clone()])
        .with_help("rename one of the duplicate IDs");

        assert_eq!(diagnostic.code.as_str(), "RECITE_ID001");
        assert_eq!(diagnostic.code.to_string(), "RECITE_ID001");
        assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);
        assert_eq!(diagnostic.span, primary);
        assert_eq!(diagnostic.related, vec![related]);
        assert_eq!(
            diagnostic.help.as_deref(),
            Some("rename one of the duplicate IDs")
        );
    }

    #[test]
    fn id_wrappers_are_explicit_and_display_their_inner_value() {
        let line_id = LineId::new("tavern_intro_001");
        let same_line_id = LineId::from("tavern_intro_001");
        let choice_id = ChoiceId::new("ask_for_room");

        assert_eq!(line_id, same_line_id);
        assert_eq!(line_id.as_str(), "tavern_intro_001");
        assert_eq!(line_id.to_string(), "tavern_intro_001");
        assert_eq!(choice_id.as_str(), "ask_for_room");
    }

    #[test]
    fn metadata_preserves_source_order_and_repeated_keys() {
        let mut metadata = Metadata::new();
        metadata.push(MetadataEntry::new("sfx", ScalarValue::from("door")));
        metadata.push(MetadataEntry::new("portrait", ScalarValue::from("neutral")));
        metadata.push(
            MetadataEntry::new("sfx", ScalarValue::from("mug")).with_source_span(
                SourceSpan::point("dialogue/tavern.recite", SourcePosition::new(4, 9)),
            ),
        );

        let keys = metadata
            .iter()
            .map(|entry| entry.key.as_str())
            .collect::<Vec<_>>();

        assert_eq!(keys, ["sfx", "portrait", "sfx"]);
        assert_eq!(metadata.len(), 3);
        assert!(!metadata.is_empty());
        assert!(metadata.as_slice()[2].source_span.is_some());
    }

    #[test]
    fn values_support_scalars_and_arrays_of_scalars() {
        let values = [
            Value::from(ScalarValue::from("neutral")),
            Value::from(ScalarValue::from(3_i64)),
            Value::from(ScalarValue::from(1.5_f64)),
            Value::from(ScalarValue::from(true)),
            Value::Array(vec![
                ScalarValue::from("door"),
                ScalarValue::from("mug"),
                ScalarValue::from(false),
            ]),
        ];

        assert_eq!(
            values[0],
            Value::Scalar(ScalarValue::String("neutral".to_owned()))
        );
        assert_eq!(values[1], Value::Scalar(ScalarValue::Integer(3)));
        assert_eq!(values[2], Value::Scalar(ScalarValue::Float(1.5)));
        assert_eq!(values[3], Value::Scalar(ScalarValue::Boolean(true)));
        assert_eq!(
            values[4],
            Value::Array(vec![
                ScalarValue::String("door".to_owned()),
                ScalarValue::String("mug".to_owned()),
                ScalarValue::Boolean(false),
            ])
        );
    }
}
