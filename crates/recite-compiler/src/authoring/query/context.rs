use recite_core::{DocumentKey, MetadataTarget, SourcePosition, SourceSpan};

use super::types::{BlockTarget, ClauseKind};

mod position;
mod site;
use position::{assignment_span, byte_at_column, line_at, span, token_span};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Site {
    Blocks {
        target: BlockTarget,
        token: SourceSpan,
    },
    Speakers(SourceSpan),
    MetadataKey {
        span: SourceSpan,
        target: MetadataTarget,
    },
    MetadataValue {
        key: String,
        value: String,
        token: SourceSpan,
        target: MetadataTarget,
    },
    Conditions {
        span: SourceSpan,
        clause: Option<(ClauseKind, SourceSpan)>,
    },
    Effects(SourceSpan),
    AvailabilityReasons {
        value: String,
        token: SourceSpan,
    },
}

pub(super) fn at(key: &DocumentKey, text: &str, position: SourcePosition) -> Option<Site> {
    let (line, line_start) = line_at(text, position.line())?;
    let cursor = byte_at_column(line, position.column());
    let prefix = &line[..cursor];
    let trimmed = prefix.trim_start();
    let trim_offset = prefix.len() - trimmed.len();
    let line_trimmed = line.trim_start_matches([' ', '\t']);
    let line_trim_offset = line.len() - line_trimmed.len();

    if trimmed.starts_with("->") {
        let mut start = trim_offset + 2;
        while line
            .get(start..)?
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
        {
            start += line[start..].chars().next()?.len_utf8();
        }
        let token = span(key, text, line_start + start, line_start + cursor);
        let raw = line.get(start..cursor)?.trim();
        let target = match raw.rsplit_once("::") {
            Some((file, _)) => match DocumentKey::new(file.to_owned()) {
                Ok(file) => BlockTarget::Qualified(file),
                Err(_) => BlockTarget::InvalidQualified {
                    target: file.to_owned(),
                },
            },
            None => BlockTarget::Local,
        };
        return Some(Site::Blocks { target, token });
    }
    if trimmed.starts_with('?')
        && let Some(assignment) = assignment_covering(line, cursor)
    {
        if assignment.key == "requires" {
            let clause = assignment_span(key, text, line, line_start, assignment);
            return Some(Site::Conditions {
                span: token_span(key, text, line, line_start, assignment.value_start, cursor),
                clause: Some((ClauseKind::Requires, clause)),
            });
        }
        if assignment.key == "reason" {
            return Some(Site::AvailabilityReasons {
                value: assignment.value.to_owned(),
                token: span(
                    key,
                    text,
                    line_start + assignment.value_start,
                    line_start + assignment.end,
                ),
            });
        }
    }
    if let Some(marker) = recite_parser::condition_marker(line_trimmed) {
        let marker_start = line_trim_offset;
        let marker_end = marker_start + marker.text().len();
        let rest = &line_trimmed[marker.text().len()..];
        let whitespace_len = rest.len() - rest.trim_start_matches(char::is_whitespace).len();
        let start = marker_end + whitespace_len;
        let clause = (matches!(marker, recite_parser::ConditionMarker::If)
            && marker_start <= cursor
            && cursor < marker_end)
            .then(|| {
                (
                    ClauseKind::If,
                    span(
                        key,
                        text,
                        line_start + marker_start,
                        line_start + marker_end,
                    ),
                )
            });
        return Some(Site::Conditions {
            span: token_span(key, text, line, line_start, start, cursor),
            clause,
        });
    }
    if trimmed.starts_with("!") {
        let start = trim_offset + 1;
        return Some(Site::Effects(token_span(
            key, text, line, line_start, start, cursor,
        )));
    }
    if let Some(assignment) = assignment_at(line, cursor) {
        let token_start = line_start + assignment.value_start;
        let token = span(key, text, token_start, line_start + cursor);
        if assignment.key == "speaker" && !trimmed.starts_with('?') {
            return Some(Site::Speakers(token));
        }
        return Some(Site::MetadataValue {
            key: assignment.key.to_owned(),
            value: assignment.value.to_owned(),
            token,
            target: metadata_target(trimmed),
        });
    }
    let token = prefix.split_whitespace().last().unwrap_or_default();
    let token_start = cursor.saturating_sub(token.len());
    if !(token.is_empty()
        || token.contains('=')
        || (trimmed.starts_with("::") && "default".starts_with(token)))
        && matches!(trimmed.as_bytes().first(), Some(b':' | b'>' | b'?'))
        && prefix.split_whitespace().count() >= 3
    {
        return Some(Site::MetadataKey {
            span: span(key, text, line_start + token_start, line_start + cursor),
            target: metadata_target(trimmed),
        });
    }
    None
}

fn assignment_at(line: &str, cursor: usize) -> Option<recite_parser::MetadataAssignment<'_>> {
    recite_parser::metadata_assignment_at(line, cursor)
}

fn assignment_covering(line: &str, cursor: usize) -> Option<recite_parser::MetadataAssignment<'_>> {
    recite_parser::metadata_assignments(line)
        .into_iter()
        .find(|assignment| {
            let key_start = assignment
                .value_start
                .saturating_sub(assignment.key.len() + 1);
            key_start <= cursor && cursor <= assignment.end
        })
}

fn metadata_target(line: &str) -> MetadataTarget {
    match line.as_bytes().first() {
        Some(b':') => MetadataTarget::Block,
        Some(b'?') => MetadataTarget::Choice,
        _ => MetadataTarget::Line,
    }
}

pub(super) fn token_at(
    key: &DocumentKey,
    text: &str,
    position: SourcePosition,
) -> Option<(String, SourceSpan)> {
    let (line, line_start) = line_at(text, position.line())?;
    let cursor = byte_at_column(line, position.column());
    let is_token =
        |character: char| character.is_alphanumeric() || matches!(character, '_' | '.' | '-');
    let mut start = cursor;
    while start > 0 {
        let (index, character) = line[..start].char_indices().next_back()?;
        if !is_token(character) {
            break;
        }
        start = index;
    }
    let mut end = cursor;
    while let Some(character) = line.get(end..)?.chars().next() {
        if !is_token(character) {
            break;
        }
        end += character.len_utf8();
    }
    if start == end {
        return None;
    }
    Some((
        line[start..end].to_owned(),
        span(key, text, line_start + start, line_start + end),
    ))
}
