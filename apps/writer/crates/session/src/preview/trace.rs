//! Borrowed inspection of the running trace; pagination carries no publication authority.
mod browser;
pub use browser::{
    Browser, FIELDS, FieldPreview, ListPage, Place, Projection, RECORDS, RecordPage, ValuePage,
    ValuePosition,
};
mod error;
mod event;
mod lookup;
mod values;
use recite_runtime::{
    LocalizedLookupTrace, PluralLineTrace,
    preview::{PreviewEvent, PreviewTrace},
};
use std::borrow::Cow;
pub use values::{condition, effect};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Section {
    #[default]
    Events,
    Localisation,
    Plurals,
}
impl Section {
    pub fn caption(self) -> &'static str {
        match self {
            Self::Events => "Events",
            Self::Localisation => "Translation lookups",
            Self::Plurals => "Plural lookups",
        }
    }
    pub fn len(self, trace: &PreviewTrace) -> usize {
        match self {
            Self::Events => trace.events().len(),
            Self::Localisation => trace.localized_lookups().len(),
            Self::Plurals => trace.plural_lines().len(),
        }
    }
    pub fn record(self, trace: &PreviewTrace, index: usize) -> Option<Record<'_>> {
        match self {
            Self::Events => trace.events().get(index).map(Record::Event),
            Self::Localisation => trace
                .localized_lookups()
                .nth(index)
                .map(Record::Localisation),
            Self::Plurals => trace
                .plural_lines()
                .nth(index)
                .map(|(id, lookup)| Record::Plural(id, lookup)),
        }
    }
}
pub enum Record<'a> {
    Event(&'a PreviewEvent),
    Localisation(&'a LocalizedLookupTrace),
    Plural(&'a str, &'a PluralLineTrace),
}
impl<'a> Record<'a> {
    /// A bounded list caption; detailed values remain separately inspectable.
    pub fn summary(&self) -> String {
        let subject = match self {
            Self::Event(event) => event::subject(event),
            Self::Localisation(lookup) => lookup.source_text.as_str(),
            Self::Plural(_, lookup) => lookup.plural_source_text.as_str(),
        };
        let subject: String = subject.chars().take(120).collect();
        if subject.is_empty() {
            self.caption().into()
        } else {
            format!("{} · {subject}", self.caption())
        }
    }
    pub fn caption(&self) -> &'static str {
        match self {
            Self::Event(event) => event::caption(event),
            Self::Localisation(_) => "Translation lookup",
            Self::Plural(_, _) => "Plural lookup",
        }
    }
    pub fn field_count(&self) -> usize {
        match self {
            Self::Event(event) => event::fields(event),
            Self::Localisation(lookup) => 9 + lookup.attempts.len() * 4,
            Self::Plural(_, lookup) => 11 + lookup.attempts.len() * 5,
        }
    }
    pub fn field(&self, index: usize) -> Option<Field<'a>> {
        match self {
            Self::Event(event) => event::field(event, index),
            Self::Localisation(lookup) => lookup::singular(lookup, index),
            Self::Plural(id, lookup) => lookup::plural(id, lookup, index),
        }
    }
}
pub struct Field<'a> {
    pub name: Cow<'static, str>,
    value: Value<'a>,
}
enum Value<'a> {
    Text(Cow<'a, str>),
    Hex(&'a [u8]),
}
impl<'a> Field<'a> {
    fn text(name: impl Into<Cow<'static, str>>, value: impl Into<Cow<'a, str>>) -> Self {
        Self {
            name: name.into(),
            value: Value::Text(value.into()),
        }
    }
    fn hex(name: &'static str, bytes: &'a [u8]) -> Self {
        Self {
            name: name.into(),
            value: Value::Hex(bytes),
        }
    }
    fn display(name: &'static str, value: impl std::fmt::Display) -> Self {
        Self::text(name, value.to_string())
    }
    fn optional(name: &'static str, value: Option<&'a str>) -> Self {
        Self::text(name, value.unwrap_or("None"))
    }
    fn argument(mut self, ordinal: usize) -> Self {
        self.name = format!("Argument {ordinal} · {}", self.name).into();
        self
    }
    fn excerpt(&self) -> (String, bool) {
        match &self.value {
            Value::Hex(bytes) => (hex(&bytes[..bytes.len().min(80)]), bytes.len() > 80),
            Value::Text(text) => {
                let mut end = text.len().min(160);
                while !text.is_char_boundary(end) {
                    end -= 1;
                }
                (text[..end].into(), end < text.len())
            }
        }
    }
    /// Byte windows seek directly into long UTF-8 text, adjusting at most three bytes at each edge.
    pub fn part(&self, part: usize) -> Part {
        match &self.value {
            Value::Text(text) => self.text_part(text, part),
            Value::Hex(bytes) => {
                let parts = bytes.len().div_ceil(500).max(1);
                let part = part.min(parts - 1);
                let start = part * 500;
                let end = start + (bytes.len() - start).min(500);
                Part {
                    name: self.name.clone(),
                    value: hex(&bytes[start..end]),
                    part,
                    parts,
                }
            }
        }
    }
    fn text_part(&self, text: &str, part: usize) -> Part {
        const BYTES: usize = 1_000;
        let parts = text.len().div_ceil(BYTES).max(1);
        let part = part.min(parts - 1);
        let mut start = part * BYTES;
        let mut end = ((part + 1) * BYTES).min(text.len());
        while !text.is_char_boundary(start) {
            start -= 1;
        }
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        Part {
            name: self.name.clone(),
            value: text[start..end].into(),
            part,
            parts,
        }
    }
}
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    output
}
pub struct Part {
    pub name: Cow<'static, str>,
    pub value: String,
    pub part: usize,
    pub parts: usize,
}

#[cfg(test)]
mod tests;
