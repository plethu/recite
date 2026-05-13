use recite_core::{SourcePosition, SourceSpan};

#[derive(Clone, Copy, Debug)]
pub(crate) struct LogicalLine<'a> {
    pub(crate) number: u32,
    pub(crate) text: &'a str,
    pub(crate) newline: &'a str,
}

impl<'a> LogicalLine<'a> {
    pub(crate) fn content_without_newline(self) -> &'a str {
        self.text
    }

    pub(crate) fn indent_len(self) -> usize {
        indent_len(self.content_without_newline())
    }

    pub(crate) fn indentation(self) -> &'a str {
        &self.text[..self.indent_len()]
    }

    pub(crate) fn trimmed_content(self) -> &'a str {
        &self.text[self.indent_len()..]
    }
}

pub(crate) struct LogicalLines<'a> {
    source: &'a str,
    offset: usize,
    number: u32,
}

impl<'a> LogicalLines<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Self {
            source,
            offset: 0,
            number: 1,
        }
    }
}

impl<'a> Iterator for LogicalLines<'a> {
    type Item = LogicalLine<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.source.len() {
            return None;
        }

        let remaining = &self.source[self.offset..];
        let newline_offset = remaining.find('\n');
        let (line_text, newline, advance) = match newline_offset {
            Some(offset) => {
                let line_end = self.offset + offset;
                let newline_start =
                    if line_end > 0 && self.source.as_bytes().get(line_end - 1) == Some(&b'\r') {
                        line_end - 1
                    } else {
                        line_end
                    };

                (
                    &self.source[self.offset..newline_start],
                    &self.source[newline_start..=line_end],
                    offset + 1,
                )
            }
            None => (remaining, "", remaining.len()),
        };

        let line = LogicalLine {
            number: self.number,
            text: line_text,
            newline,
        };
        self.offset += advance;
        self.number += 1;

        Some(line)
    }
}

pub(crate) fn indent_len(content: &str) -> usize {
    content.len() - content.trim_start_matches([' ', '\t']).len()
}

pub(crate) fn span_for_line(path: &str, line: u32, column: usize) -> SourceSpan {
    SourceSpan::point(
        path,
        SourcePosition::new(line, u32::try_from(column).unwrap_or(u32::MAX))
            .expect("parser line and column positions are 1-based"),
    )
}

pub(crate) fn span_for_text(path: &str, line: u32, column: usize, text: &str) -> SourceSpan {
    if text.is_empty() {
        return span_for_line(path, line, column);
    }

    let end_column = column
        .saturating_add(text.chars().count())
        .saturating_sub(1);
    SourceSpan::new(
        path,
        SourcePosition::new(line, u32::try_from(column).unwrap_or(u32::MAX))
            .expect("parser line and column positions are 1-based"),
        Some(
            SourcePosition::new(line, u32::try_from(end_column).unwrap_or(u32::MAX))
                .expect("parser line and column positions are 1-based"),
        ),
    )
}
