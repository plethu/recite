use recite_core::{ScalarValue, SourceSpan, Value};

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
        let mut quote = None;
        let mut bracket_depth = 0_u32;

        while let Some(character) = self.current_char() {
            match character {
                '\\' if quote.is_some() => {
                    self.advance_char();
                    self.advance_char();
                }
                '"' if quote == Some('"') => {
                    quote = None;
                    self.advance_char();
                }
                '"' if quote.is_none() => {
                    quote = Some('"');
                    self.advance_char();
                }
                '[' if quote.is_none() => {
                    bracket_depth += 1;
                    self.advance_char();
                }
                ']' if quote.is_none() && bracket_depth > 0 => {
                    bracket_depth -= 1;
                    self.advance_char();
                }
                ' ' | '\t' if quote.is_none() && bracket_depth == 0 => break,
                _ => self.advance_char(),
            }
        }

        Some(HeaderField {
            text: &self.trimmed[start..self.cursor],
            line: self.line,
            column: self.base_column + self.trimmed[..start].chars().count(),
            offset: start,
        })
    }
}

impl HeaderFields<'_> {
    fn current_char(&self) -> Option<char> {
        self.trimmed[self.cursor..].chars().next()
    }

    fn advance_char(&mut self) {
        let Some(character) = self.current_char() else {
            return;
        };
        self.cursor += character.len_utf8();
    }
}

fn parse_value(value: &str) -> Result<Value, ()> {
    if value.starts_with('[') {
        return parse_array(value).map(Value::Array);
    }

    parse_scalar(value).map(Value::Scalar)
}

fn parse_array(value: &str) -> Result<Vec<ScalarValue>, ()> {
    let inner = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or(())?;
    let trimmed = inner.trim();

    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    split_array_items(trimmed)?
        .into_iter()
        .map(|item| parse_scalar(item.trim()))
        .collect()
}

fn split_array_items(value: &str) -> Result<Vec<&str>, ()> {
    let mut items = Vec::new();
    let mut quote = None;
    let mut start = 0;
    let mut cursor = 0;

    while let Some(character) = value[cursor..].chars().next() {
        match character {
            '\\' if quote.is_some() => {
                cursor += character.len_utf8();
                if let Some(escaped) = value[cursor..].chars().next() {
                    cursor += escaped.len_utf8();
                }
            }
            '"' if quote == Some('"') => {
                quote = None;
                cursor += character.len_utf8();
            }
            '"' if quote.is_none() => {
                quote = Some('"');
                cursor += character.len_utf8();
            }
            ',' if quote.is_none() => {
                let item = value[start..cursor].trim();
                if item.is_empty() {
                    return Err(());
                }
                items.push(item);
                cursor += character.len_utf8();
                start = cursor;
            }
            _ => cursor += character.len_utf8(),
        }
    }

    if quote.is_some() {
        return Err(());
    }

    let item = value[start..].trim();
    if item.is_empty() {
        return Err(());
    }
    items.push(item);
    Ok(items)
}

fn parse_scalar(value: &str) -> Result<ScalarValue, ()> {
    if value == "true" {
        return Ok(ScalarValue::Boolean(true));
    }

    if value == "false" {
        return Ok(ScalarValue::Boolean(false));
    }

    if value.starts_with('"') {
        return unquote(value).map(ScalarValue::String);
    }

    if value.starts_with('[') || value.ends_with(']') || value.contains('"') {
        return Err(());
    }

    if let Ok(integer) = value.parse::<i64>() {
        return Ok(ScalarValue::Integer(integer));
    }

    if let Ok(float) = value.parse::<f64>() {
        return Ok(ScalarValue::Float(float));
    }

    Ok(ScalarValue::String(value.to_owned()))
}

fn unquote(value: &str) -> Result<String, ()> {
    let inner = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or(())?;
    let mut output = String::new();
    let mut chars = inner.chars();

    while let Some(character) = chars.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }

        let Some(escaped) = chars.next() else {
            return Err(());
        };
        match escaped {
            '"' => output.push('"'),
            '\\' => output.push('\\'),
            'n' => output.push('\n'),
            't' => output.push('\t'),
            other => output.push(other),
        }
    }

    Ok(output)
}
