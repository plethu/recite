//! Moves dialogue an author typed on a line or choice header into that statement's body.
use recite_core::{DocumentKey, SourcePosition, source_lines};

use super::helpers::{document, make_plan, no_symbol};
use super::{AuthoringEditError, AuthoringEditOperation, AuthoringEditPlan, SourceEdit};
use crate::authoring::{AuthoringQuery, SourceRange};

/// The parser's code for dialogue written on a header (`RECITE_PARSE019`).
const HEADER_PROSE: &str = "RECITE_PARSE019";
/// Body text sits this many columns inside its header, as the language guide writes it.
const BODY_INDENT: usize = 2;

/// Plans the move for the header-prose diagnostic a range touches: the words leave the header,
/// which keeps its fields and `@id`, and become the first body line under it.
pub(super) fn plan_move_header_prose_in_range(
    snapshot: &AuthoringQuery<'_>,
    key: &DocumentKey,
    range: SourceRange,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot.checkpoint()?;
    let target = document(snapshot, key)?;
    let span = target
        .diagnostics()
        .iter()
        .find(|diagnostic| {
            diagnostic.code.as_str() == HEADER_PROSE
                && diagnostic.span.file == key.as_str()
                && touches(&diagnostic.span, range)
        })
        .map(|diagnostic| diagnostic.span.clone())
        .ok_or_else(|| no_symbol(key, range.start()))?;
    let end = span.end.ok_or_else(|| no_symbol(key, range.start()))?;
    let source = target.source_text();
    let line_number = span.start.line();
    let (content, _) = source_lines(source)
        .nth(
            usize::try_from(line_number)
                .unwrap_or(usize::MAX)
                .saturating_sub(1),
        )
        .ok_or_else(|| no_symbol(key, range.start()))?;
    let characters = content.chars().collect::<Vec<_>>();
    let first = column_index(span.start.column());
    let last = column_index(end.column());
    if first > last || last >= characters.len() {
        return Err(no_symbol(key, range.start()));
    }
    let prose = characters[first..=last].iter().collect::<String>();
    // Keep the header's fields separated by one space once the words leave it.
    let before = characters[..first].iter().collect::<String>();
    let after = characters[last + 1..].iter().collect::<String>();
    let header = format!("{} {}", before.trim_end(), after.trim_start());
    let header = header.trim_end();
    let indent = content.len() - content.trim_start().len();
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let replacement = format!(
        "{header}{newline}{}{prose}",
        " ".repeat(indent + BODY_INDENT)
    );
    let line_start = position(key, line_number, 1)?;
    let line_end = position(key, line_number, characters.len() + 1)?;
    make_plan(
        snapshot,
        [key.clone()],
        vec![SourceEdit::new(
            key.clone(),
            SourceRange::new(line_start, line_end),
            replacement,
        )],
        AuthoringEditOperation::MoveHeaderProse {
            document: key.clone(),
            line: line_number,
        },
    )
}

impl AuthoringQuery<'_> {
    /// Plans moving header dialogue into the body for the diagnostic a range touches.
    pub fn plan_move_header_prose_in_range(
        &self,
        key: &DocumentKey,
        range: SourceRange,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_move_header_prose_in_range(self, key, range)
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
