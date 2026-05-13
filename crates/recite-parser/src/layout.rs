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
