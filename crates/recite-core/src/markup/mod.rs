//! Shared inline-markup parsing and validation policy.
//!
//! The compiler and catalogue boundary use the same tag parser.  The compiler
//! validates a source value against the schema; translated values validate the
//! contract authored by their `msgid` (tag names, balance, and attributes).

use std::ops::Range;

use crate::{MarkupDefinition, ProjectSchema};

mod translation;

pub use translation::{MarkupTranslationError, validate_markup_translation};

/// Whether a parsed markup tag opens or closes a tagged span.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MarkupTagKind {
    Open,
    Close,
}

/// The structural reason an inline-markup tag is unbalanced.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarkupUnbalancedKind {
    MissingClosingBracket,
    Standalone,
    NoOpening,
    Mismatch { expected: String },
}

/// A source markup validation issue with byte ranges relative to the value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarkupValidationIssue {
    UnknownTag {
        tag: String,
        span: Range<usize>,
    },
    UnbalancedTag {
        tag: String,
        span: Range<usize>,
        kind: MarkupUnbalancedKind,
        related_opening: Option<Range<usize>>,
    },
    MissingClosingTag {
        tag: String,
        span: Range<usize>,
    },
    InvalidNesting {
        parent: String,
        child: String,
        child_span: Range<usize>,
        parent_span: Range<usize>,
    },
}

/// Validate source inline markup against the schema's declared tag policy.
pub fn validate_markup(source: &str, schema: &ProjectSchema) -> Vec<MarkupValidationIssue> {
    let parsed = parse_tags(source);
    let mut issues = parsed.issue.into_iter().collect::<Vec<_>>();
    let mut open_tags = Vec::<(&ParsedTag, &MarkupDefinition)>::new();

    for tag in &parsed.tags {
        let Some(definition) = schema.markup.get(&tag.name) else {
            issues.push(MarkupValidationIssue::UnknownTag {
                tag: tag.name.clone(),
                span: tag.span.clone(),
            });
            continue;
        };

        match tag.kind {
            MarkupTagKind::Open => {
                if let Some((parent, _)) = open_tags
                    .iter()
                    .rev()
                    .find(|(_, definition)| !definition.allows_nesting)
                {
                    issues.push(MarkupValidationIssue::InvalidNesting {
                        parent: parent.name.clone(),
                        child: tag.name.clone(),
                        child_span: tag.span.clone(),
                        parent_span: parent.span.clone(),
                    });
                }

                if definition.requires_closing {
                    open_tags.push((tag, definition));
                }
            }
            MarkupTagKind::Close => {
                if !definition.requires_closing {
                    issues.push(MarkupValidationIssue::UnbalancedTag {
                        tag: tag.name.clone(),
                        span: tag.span.clone(),
                        kind: MarkupUnbalancedKind::Standalone,
                        related_opening: None,
                    });
                    continue;
                }

                let Some((opening, _)) = open_tags.last() else {
                    issues.push(MarkupValidationIssue::UnbalancedTag {
                        tag: tag.name.clone(),
                        span: tag.span.clone(),
                        kind: MarkupUnbalancedKind::NoOpening,
                        related_opening: None,
                    });
                    continue;
                };

                if opening.name == tag.name {
                    open_tags.pop();
                } else {
                    issues.push(MarkupValidationIssue::UnbalancedTag {
                        tag: tag.name.clone(),
                        span: tag.span.clone(),
                        kind: MarkupUnbalancedKind::Mismatch {
                            expected: opening.name.clone(),
                        },
                        related_opening: Some(opening.span.clone()),
                    });
                }
            }
        }
    }

    issues.extend(
        open_tags
            .into_iter()
            .map(|(tag, _)| MarkupValidationIssue::MissingClosingTag {
                tag: tag.name.clone(),
                span: tag.span.clone(),
            }),
    );
    issues
}

#[derive(Clone, Debug)]
struct ParsedTag {
    name: String,
    kind: MarkupTagKind,
    attributes: String,
    span: Range<usize>,
}

struct ParsedTags {
    tags: Vec<ParsedTag>,
    issue: Option<MarkupValidationIssue>,
}

fn parse_tags(source: &str) -> ParsedTags {
    let mut tags = Vec::new();
    let mut offset = 0;
    let mut issue = None;
    while let Some(relative_open) = source[offset..].find('[') {
        let open = offset + relative_open;
        let Some(relative_close) = source[open..].find(']') else {
            issue = Some(MarkupValidationIssue::UnbalancedTag {
                tag: "[".to_owned(),
                span: open..open + 1,
                kind: MarkupUnbalancedKind::MissingClosingBracket,
                related_opening: None,
            });
            break;
        };
        let close = open + relative_close;
        offset = close + 1;
        let raw_inner = &source[open + 1..close];
        let leading_ws = raw_inner.len() - raw_inner.trim_start().len();
        let inner = raw_inner.trim();
        let (kind, name_start, name_source) =
            inner
                .strip_prefix('/')
                .map_or((MarkupTagKind::Open, 0, inner), |closing| {
                    let trimmed = closing.trim_start();
                    (
                        MarkupTagKind::Close,
                        1 + closing.len() - trimmed.len(),
                        trimmed,
                    )
                });
        let name_len = name_source
            .char_indices()
            .find_map(|(index, ch)| (!is_tag_name_char(ch)).then_some(index))
            .unwrap_or(name_source.len());
        if name_len == 0 {
            continue;
        }
        let name_offset = open + 1 + leading_ws + name_start;
        let attributes = name_source[name_len..].trim().to_owned();
        tags.push(ParsedTag {
            name: name_source[..name_len].to_owned(),
            kind,
            attributes,
            span: name_offset..name_offset + name_len,
        });
    }
    ParsedTags { tags, issue }
}

fn is_tag_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}
