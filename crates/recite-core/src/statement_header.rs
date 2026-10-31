//! Canonical spelling of line and choice headers.
//!
//! Generators (importers, starter projects, fixture corpora, tests) build
//! headers here instead of formatting marker, metadata and ID placement
//! themselves, so a change to header syntax has one owner.

use std::fmt;

use crate::StableId;

/// Prefix of the stable-ID field that ends a line or choice header.
pub const STABLE_ID_FIELD_PREFIX: char = '@';

/// A stable ID spelled as its trailing header field, `@id`.
#[derive(Clone, Copy, Debug)]
pub struct StableIdField<'a>(&'a StableId);

impl fmt::Display for StableIdField<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{STABLE_ID_FIELD_PREFIX}{}", self.0)
    }
}

impl StableId {
    /// This ID as the `@id` field that ends a header.
    #[must_use]
    pub const fn field(&self) -> StableIdField<'_> {
        StableIdField(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StatementMarker {
    Line,
    Choice,
}

impl StatementMarker {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Line => ">",
            Self::Choice => "?",
        }
    }
}

/// A line (`>`) or choice (`?`) header: marker, `key=value` fields in order,
/// then the stable ID.
///
/// ```
/// use recite_core::{StableId, StatementHeader};
///
/// let id = StableId::new("desk_greeting").unwrap();
/// let header = StatementHeader::line().field("speaker", "mara").id(&id);
/// assert_eq!(header.to_string(), "> speaker=mara @desk_greeting");
/// ```
#[derive(Clone, Debug)]
#[must_use]
pub struct StatementHeader {
    marker: StatementMarker,
    fields: Vec<String>,
    id: Option<StableId>,
}

impl StatementHeader {
    pub const fn line() -> Self {
        Self::new(StatementMarker::Line)
    }

    pub const fn choice() -> Self {
        Self::new(StatementMarker::Choice)
    }

    const fn new(marker: StatementMarker) -> Self {
        Self {
            marker,
            fields: Vec::new(),
            id: None,
        }
    }

    /// Appends `key=value`. `value` is written as source text, so quoted
    /// strings and clause expressions keep their own spelling.
    pub fn field(mut self, key: &str, value: impl fmt::Display) -> Self {
        self.fields.push(format!("{key}={value}"));
        self
    }

    pub fn id(mut self, id: &StableId) -> Self {
        self.id = Some(id.clone());
        self
    }
}

impl fmt::Display for StatementHeader {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.marker.as_str())?;
        for field in &self.fields {
            write!(formatter, " {field}")?;
        }
        if let Some(id) = &self.id {
            write!(formatter, " {}", id.field())?;
        }
        Ok(())
    }
}
