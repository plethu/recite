use super::field::HeaderField;

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
        column: base_column + prefix.chars().count(),
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

pub(crate) fn rest_after_field<'a>(trimmed: &'a str, field: HeaderField<'_>) -> HeaderRest<'a> {
    let start = field.offset + field.text.len();
    let rest = &trimmed[start..];
    let whitespace_len = rest.len() - rest.trim_start_matches([' ', '\t']).len();

    HeaderRest {
        text: &rest[whitespace_len..],
        column: field.column + field.text.chars().count() + whitespace_len,
    }
}

pub(crate) struct HeaderFields<'a> {
    trimmed: &'a str,
    cursor: usize,
    line: u32,
    column: usize,
}

impl<'a> Iterator for HeaderFields<'a> {
    type Item = HeaderField<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.cursor < self.trimmed.len()
            && matches!(self.trimmed.as_bytes()[self.cursor], b' ' | b'\t')
        {
            self.cursor += 1;
            self.column += 1;
        }

        if self.cursor >= self.trimmed.len() {
            return None;
        }

        let start = self.cursor;
        let column = self.column;
        let mut quote = None;
        let mut bracket_depth = 0_u32;
        let mut paren_depth = 0_u32;

        while let Some(character) = self.current_char() {
            if matches!(character, ' ' | '\t')
                && quote.is_none()
                && bracket_depth == 0
                && paren_depth == 0
            {
                break;
            }

            self.advance_char(character);
            match character {
                '\\' if quote.is_some() => {
                    if let Some(escaped) = self.current_char() {
                        self.advance_char(escaped);
                    }
                }
                '"' if quote == Some('"') => quote = None,
                '"' if quote.is_none() => quote = Some('"'),
                '[' if quote.is_none() => bracket_depth += 1,
                ']' if quote.is_none() && bracket_depth > 0 => bracket_depth -= 1,
                '(' if quote.is_none() => paren_depth += 1,
                ')' if quote.is_none() && paren_depth > 0 => paren_depth -= 1,
                _ => {}
            }
        }

        Some(HeaderField {
            text: &self.trimmed[start..self.cursor],
            line: self.line,
            column,
            offset: start,
        })
    }
}

impl HeaderFields<'_> {
    fn current_char(&self) -> Option<char> {
        self.trimmed[self.cursor..].chars().next()
    }

    fn advance_char(&mut self, character: char) {
        self.cursor += character.len_utf8();
        self.column += 1;
    }
}
