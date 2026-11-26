//! Applies a diagnostic's "did you mean" suggestion: only the misspelt name changes.
use recite_core::{DiagnosticArgumentValue, DocumentKey, SourcePosition, source_lines};

use super::helpers::{document, make_plan, no_symbol};
use super::{AuthoringEditError, AuthoringEditOperation, AuthoringEditPlan, SourceEdit};
use crate::authoring::{AuthoringQuery, SourceRange};

/// Suggestion help presentations carry the replacement in their `suggestion` argument.
const SUGGESTION_SUFFIX: &str = "-help-suggestion";

/// Plans replacing the misspelt name for the suggestion-bearing diagnostic a range touches. An
/// unknown speaker's span is the name itself; an unknown block reference's span starts at its
/// divert, so the reference text is found after it on that line.
pub(super) fn plan_apply_suggestion_in_range(
    snapshot: &AuthoringQuery<'_>,
    key: &DocumentKey,
    range: SourceRange,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot.checkpoint()?;
    let target = document(snapshot, key)?;
    let refuse = || no_symbol(key, range.start());
    let (diagnostic, suggestion) = target
        .diagnostics()
        .iter()
        .filter(|diagnostic| {
            diagnostic.span.file == key.as_str() && touches(&diagnostic.span, range)
        })
        .find_map(|diagnostic| {
            let help = diagnostic.help_presentation.as_ref()?;
            if !help.id().as_str().ends_with(SUGGESTION_SUFFIX) {
                return None;
            }
            match help.arguments().get("suggestion")? {
                DiagnosticArgumentValue::String(suggestion) => {
                    Some((diagnostic, suggestion.clone()))
                }
                _ => None,
            }
        })
        .ok_or_else(refuse)?;
    let line = diagnostic.span.start.line();
    let (content, _) = source_lines(target.source_text())
        .nth(
            usize::try_from(line)
                .unwrap_or(usize::MAX)
                .saturating_sub(1),
        )
        .ok_or_else(refuse)?;
    let characters = content.chars().collect::<Vec<_>>();
    let start = column_index(diagnostic.span.start.column());
    let (first, length) = match diagnostic.span.end.filter(|end| end.line() == line) {
        Some(end) if column_index(end.column()) >= start => {
            (start, column_index(end.column()) - start + 1)
        }
        _ => {
            let reference = match diagnostic
                .presentation
                .as_ref()
                .and_then(|presentation| presentation.arguments().get("reference"))
            {
                Some(DiagnosticArgumentValue::String(reference)) => reference.clone(),
                _ => return Err(refuse()),
            };
            let tail = characters[start.min(characters.len())..]
                .iter()
                .collect::<String>();
            let offset = tail.find(&reference).ok_or_else(refuse)?;
            (
                start + tail[..offset].chars().count(),
                reference.chars().count(),
            )
        }
    };
    let begin = position(key, line, first + 1)?;
    let end = position(key, line, first + length + 1)?;
    make_plan(
        snapshot,
        [key.clone()],
        vec![SourceEdit::new(
            key.clone(),
            SourceRange::new(begin, end),
            suggestion.clone(),
        )],
        AuthoringEditOperation::ApplySuggestion {
            document: key.clone(),
            line,
            replacement: suggestion,
        },
    )
}

impl AuthoringQuery<'_> {
    /// Plans applying the "did you mean" suggestion of the diagnostic a range touches.
    pub fn plan_apply_suggestion_in_range(
        &self,
        key: &DocumentKey,
        range: SourceRange,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_apply_suggestion_in_range(self, key, range)
    }
}

fn touches(span: &recite_core::SourceSpan, range: SourceRange) -> bool {
    let end = span.end.unwrap_or(span.start);
    range.start().line() <= end.line() && range.end().line() >= span.start.line()
}

fn column_index(column: u32) -> usize {
    usize::try_from(column)
        .unwrap_or(usize::MAX)
        .saturating_sub(1)
}

fn position(
    key: &DocumentKey,
    line: u32,
    column: usize,
) -> Result<SourcePosition, AuthoringEditError> {
    SourcePosition::new(line, u32::try_from(column).unwrap_or(u32::MAX)).map_err(|_| {
        AuthoringEditError::UnmappableRange {
            document: key.clone(),
        }
    })
}
