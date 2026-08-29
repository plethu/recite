use recite_core::{MarkupValidationIssue, SourcePosition, SourceSpan, SourceText};

use super::state::Validator;
use crate::diagnostics;

impl<'a> Validator<'a> {
    pub(crate) fn validate_markup(&mut self, source_text: &'a SourceText) {
        let Some(schema) = self.schema else {
            return;
        };

        for issue in recite_core::validate_markup(&source_text.text, schema) {
            match issue {
                MarkupValidationIssue::UnknownTag { tag, span } => {
                    self.diagnostics.push(diagnostics::unknown_markup_tag(
                        &tag,
                        span_for_range(source_text, &span),
                    ));
                }
                MarkupValidationIssue::UnbalancedTag {
                    tag,
                    span,
                    kind,
                    related_opening,
                } => {
                    self.diagnostics.push(diagnostics::unbalanced_markup_tag(
                        &tag,
                        span_for_range(source_text, &span),
                        kind,
                        related_opening
                            .as_ref()
                            .map(|opening| span_for_range(source_text, opening)),
                    ));
                }
                MarkupValidationIssue::MissingClosingTag { tag, span } => {
                    self.diagnostics
                        .push(diagnostics::missing_markup_closing_tag(
                            &tag,
                            span_for_range(source_text, &span),
                        ));
                }
                MarkupValidationIssue::InvalidNesting {
                    parent,
                    child,
                    child_span,
                    parent_span,
                } => {
                    self.diagnostics.push(diagnostics::invalid_markup_nesting(
                        &parent,
                        &child,
                        span_for_range(source_text, &child_span),
                        span_for_range(source_text, &parent_span),
                    ));
                }
            }
        }
    }
}

fn span_for_range(source_text: &SourceText, range: &std::ops::Range<usize>) -> SourceSpan {
    span_for_offset(source_text, range.start, range.end - range.start)
}

fn span_for_offset(source_text: &SourceText, offset: usize, len: usize) -> SourceSpan {
    let start = position_for_offset(source_text, offset);
    let end = (len > 0).then(|| position_for_offset(source_text, offset + len));
    SourceSpan::new(source_text.span.file.clone(), start, end)
}

// Invariant: offsets are within source text whose span starts at a valid source position.
#[allow(clippy::expect_used)]
fn position_for_offset(source_text: &SourceText, offset: usize) -> SourcePosition {
    let mut line = source_text.span.start.line();
    let mut column = source_text.span.start.column();
    for ch in source_text.text[..offset].chars() {
        if ch == '\n' {
            line += 1;
            column = source_text.span.start.column();
        } else {
            column += 1;
        }
    }
    SourcePosition::new(line, column).expect("source text offsets produce valid positions")
}
