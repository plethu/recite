use recite_core::{SourceSpan, Value};

use crate::source::span_for_text;

use super::value::parse_value;

#[derive(Clone, Copy, Debug)]
pub(crate) struct HeaderField<'a> {
    pub(crate) text: &'a str,
    pub(crate) line: u32,
    pub(crate) column: usize,
    pub(crate) offset: usize,
}

impl<'a> HeaderField<'a> {
    pub(crate) fn span(self, path: &str) -> SourceSpan {
        span_for_text(path, self.line, self.column, self.text)
    }

    pub(crate) fn key_value(self, path: &str) -> Option<HeaderKeyValue<'a>> {
        let (key, value) = self.text.split_once('=')?;
        let value_column = self.column + key.chars().count() + 1;

        Some(HeaderKeyValue {
            key,
            value,
            field_span: self.span(path),
            key_span: span_for_text(path, self.line, self.column, key),
            value_span: span_for_text(path, self.line, value_column, value),
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct HeaderKeyValue<'a> {
    pub(crate) key: &'a str,
    pub(crate) value: &'a str,
    pub(crate) field_span: SourceSpan,
    pub(crate) key_span: SourceSpan,
    pub(crate) value_span: SourceSpan,
}

impl HeaderKeyValue<'_> {
    pub(crate) fn parse_value(&self) -> Result<Value, SourceSpan> {
        parse_value(self.value).map_err(|_| self.value_span.clone())
    }
}
