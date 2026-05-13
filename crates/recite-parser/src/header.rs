use recite_core::SourceSpan;

use crate::source::span_for_text;

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
        let value_column = self.column + key.len() + 1;

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

#[derive(Clone, Copy, Debug)]
pub(crate) struct HeaderRest<'a> {
    pub(crate) text: &'a str,
    pub(crate) column: usize,
}

pub(crate) fn fields_after_prefix<'a>(
    trimmed: &'a str,
    prefix: &str,
    line: u32,
    base_column: usize,
) -> HeaderFields<'a> {
    HeaderFields {
        trimmed,
        cursor: prefix.len(),
        line,
        base_column,
    }
}

pub(crate) fn rest_after_prefix<'a>(
    trimmed: &'a str,
    prefix: &str,
    base_column: usize,
) -> HeaderRest<'a> {
    let rest = &trimmed[prefix.len()..];
    let whitespace_len = rest.len() - rest.trim_start_matches([' ', '\t']).len();

    HeaderRest {
        text: &rest[whitespace_len..],
        column: base_column + prefix.len() + whitespace_len,
    }
}

pub(crate) struct HeaderFields<'a> {
    trimmed: &'a str,
    cursor: usize,
    line: u32,
    base_column: usize,
}

impl<'a> Iterator for HeaderFields<'a> {
    type Item = HeaderField<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.cursor < self.trimmed.len()
            && matches!(self.trimmed.as_bytes()[self.cursor], b' ' | b'\t')
        {
            self.cursor += 1;
        }

        if self.cursor >= self.trimmed.len() {
            return None;
        }

        let start = self.cursor;
        while self.cursor < self.trimmed.len()
            && !matches!(self.trimmed.as_bytes()[self.cursor], b' ' | b'\t')
        {
            self.cursor += 1;
        }

        Some(HeaderField {
            text: &self.trimmed[start..self.cursor],
            line: self.line,
            column: self.base_column + self.trimmed[..start].chars().count(),
            offset: start,
        })
    }
}
