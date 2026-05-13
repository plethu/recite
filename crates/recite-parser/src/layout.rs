use crate::markers::StatementMarker;
use crate::source::LogicalLine;
use crate::syntax::ReciteSyntaxKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ClassifiedLine {
    Blank,
    Prose,
    Statement(StatementMarker),
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LineBodyItem {
    Blank { index: usize },
    Prose { index: usize },
    MixedIndent { index: usize },
    NestedStatement { index: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LineBodyScan {
    pub(crate) items: Vec<LineBodyItem>,
    pub(crate) next_index: usize,
}

impl ClassifiedLine {
    pub(crate) const fn syntax_kind(self) -> ReciteSyntaxKind {
        match self {
            Self::Blank | Self::Prose => ReciteSyntaxKind::Prose,
            Self::Statement(marker) => marker.syntax_kind(),
            Self::Error => ReciteSyntaxKind::Error,
        }
    }
}

pub(crate) fn classify_line(line: LogicalLine<'_>) -> ClassifiedLine {
    classify_trimmed(line.trimmed_content(), line.indent_len())
}

pub(crate) fn classify_trimmed(trimmed: &str, indent_len: usize) -> ClassifiedLine {
    if trimmed.is_empty() {
        return ClassifiedLine::Blank;
    }

    if let Some(marker) = StatementMarker::parse(trimmed) {
        return ClassifiedLine::Statement(marker);
    }

    if indent_len > 0 {
        ClassifiedLine::Prose
    } else {
        ClassifiedLine::Error
    }
}

pub(crate) fn line_starts_statement(line: LogicalLine<'_>) -> bool {
    matches!(classify_line(line), ClassifiedLine::Statement(_))
}

pub(crate) fn is_body_boundary(line: LogicalLine<'_>, header_indent: usize) -> bool {
    line_starts_statement(line) && line.indent_len() <= header_indent
}

pub(crate) fn has_mixed_body_indent(
    line: LogicalLine<'_>,
    header_indent: usize,
    body_indent: usize,
) -> bool {
    let indent = line.indent_len();

    !line.trimmed_content().is_empty() && indent > header_indent && indent != body_indent
}

pub(crate) fn scan_line_body(lines: &[LogicalLine<'_>], header_index: usize) -> LineBodyScan {
    let header_indent = lines[header_index].indent_len();
    let mut body_indent = None;
    let mut items = Vec::new();
    let mut index = header_index + 1;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trimmed_content();
        let indent = line.indent_len();

        if is_body_boundary(line, header_indent) {
            break;
        }

        if trimmed.is_empty() {
            items.push(LineBodyItem::Blank { index });
            index += 1;
            continue;
        }

        let Some(expected_indent) = body_indent else {
            if indent <= header_indent {
                break;
            }

            body_indent = Some(indent);
            if line_starts_statement(line) {
                items.push(LineBodyItem::NestedStatement { index });
                index = skip_line_body_tail(lines, index, header_indent);
                break;
            }

            items.push(LineBodyItem::Prose { index });
            index += 1;
            continue;
        };

        if has_mixed_body_indent(line, header_indent, expected_indent) {
            items.push(LineBodyItem::MixedIndent { index });
            index += 1;
            continue;
        }

        if line_starts_statement(line) {
            items.push(LineBodyItem::NestedStatement { index });
            index = skip_line_body_tail(lines, index, header_indent);
            break;
        }

        items.push(LineBodyItem::Prose { index });
        index += 1;
    }

    LineBodyScan {
        items,
        next_index: index,
    }
}

fn skip_line_body_tail(lines: &[LogicalLine<'_>], mut index: usize, header_indent: usize) -> usize {
    while index < lines.len() {
        let line = lines[index];

        if is_body_boundary(line, header_indent) {
            break;
        }

        if !line.trimmed_content().is_empty() && line.indent_len() <= header_indent {
            break;
        }

        index += 1;
    }

    index
}
