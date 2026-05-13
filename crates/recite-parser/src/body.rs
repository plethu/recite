use recite_core::Diagnostic;

use crate::diagnostics::mixed_indent;
use crate::layout::{ClassifiedLine, classify_line};
use crate::markers::StatementMarker;
use crate::source::{LogicalLine, span_for_line};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BodyBoundary {
    NextBlock,
    HeaderIndent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BodyStep {
    Blank,
    Boundary,
    MixedIndent,
    Content { index: usize },
}

#[derive(Clone, Debug)]
pub(crate) struct BodyCursor {
    boundary: BodyBoundary,
    header_indent: usize,
    body_indent: Option<usize>,
    index: usize,
}

impl BodyCursor {
    pub(crate) fn new(
        lines: &[LogicalLine<'_>],
        header_index: usize,
        boundary: BodyBoundary,
    ) -> Self {
        Self {
            boundary,
            header_indent: lines[header_index].indent_len(),
            body_indent: None,
            index: header_index + 1,
        }
    }

    pub(crate) fn index(&self) -> usize {
        self.index
    }

    pub(crate) fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    pub(crate) fn advance(&mut self) {
        self.index += 1;
    }

    pub(crate) fn step(
        &mut self,
        path: &str,
        line: LogicalLine<'_>,
        emit_mixed_indent: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> BodyStep {
        let trimmed = line.trimmed_content();

        if trimmed.is_empty() {
            self.advance();
            return BodyStep::Blank;
        }

        if self.is_boundary(line) {
            return BodyStep::Boundary;
        }

        let indent = line.indent_len();
        match self.body_indent {
            Some(expected) if indent != expected => {
                if emit_mixed_indent {
                    diagnostics.push(mixed_indent(span_for_line(path, line.number, indent + 1)));
                }
                self.advance();
                BodyStep::MixedIndent
            }
            None => {
                self.body_indent = Some(indent);
                BodyStep::Content { index: self.index }
            }
            _ => BodyStep::Content { index: self.index },
        }
    }

    fn is_boundary(&self, line: LogicalLine<'_>) -> bool {
        match self.boundary {
            BodyBoundary::NextBlock => {
                classify_line(line) == ClassifiedLine::Statement(StatementMarker::Block)
            }
            BodyBoundary::HeaderIndent => line.indent_len() <= self.header_indent,
        }
    }
}
