use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use super::{MarkupTagKind, MarkupUnbalancedKind, MarkupValidationIssue, ParsedTag, parse_tags};

/// A translated value violates the inline-markup contract authored by its
/// source value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarkupTranslationError {
    NewTag {
        tag: String,
        span: Range<usize>,
    },
    MissingTag {
        tag: String,
    },
    UnbalancedTag {
        tag: String,
        span: Range<usize>,
        kind: MarkupUnbalancedKind,
        related_opening: Option<Range<usize>>,
    },
    AttributeChange {
        tag: String,
        expected: String,
        actual: String,
        span: Range<usize>,
    },
}

/// Validate translated markup against the tags authored in `source`.
///
/// Text may be reordered by a translator. Tag occurrences remain required,
/// balanced, and attribute-stable; a translation cannot introduce a new tag.
pub fn validate_markup_translation(
    source: &str,
    translation: &str,
) -> Result<(), MarkupTranslationError> {
    let source_tags = parse_tags(source);
    // The source/compiler boundary owns source validity. A malformed msgid is
    // not made worse by the catalogue parser attempting a second diagnosis.
    if source_tags.issue.is_some() {
        return Ok(());
    }
    let translated = parse_tags(translation);
    if let Some(issue) = translated.issue {
        return Err(translation_error_from_issue(issue));
    }

    let source_names = source_tags
        .tags
        .iter()
        .map(|tag| (tag.kind, tag.name.clone()))
        .collect::<BTreeSet<_>>();
    let mut source_counts = BTreeMap::<(MarkupTagKind, String), usize>::new();
    let mut translated_counts = BTreeMap::<(MarkupTagKind, String), usize>::new();
    let closing_names = source_tags
        .tags
        .iter()
        .filter(|tag| tag.kind == MarkupTagKind::Close)
        .map(|tag| tag.name.clone())
        .collect::<BTreeSet<_>>();

    for tag in &source_tags.tags {
        *source_counts
            .entry((tag.kind, tag.name.clone()))
            .or_default() += 1;
    }
    for tag in &translated.tags {
        let key = (tag.kind, tag.name.clone());
        let count = translated_counts.entry(key.clone()).or_default();
        *count += 1;
        if !source_names.contains(&key) || *count > source_counts[&key] {
            return Err(MarkupTranslationError::NewTag {
                tag: tag.name.clone(),
                span: tag.span.clone(),
            });
        }
    }

    validate_translation_balance(&translated.tags, &closing_names)?;

    for (key, expected) in &source_counts {
        let actual = translated_counts.get(key).copied().unwrap_or_default();
        if actual < *expected {
            return Err(MarkupTranslationError::MissingTag { tag: key.1.clone() });
        }
    }

    let source_attributes = structural_attributes(&source_tags.tags, &closing_names);
    let translated_attributes = structural_attributes(&translated.tags, &closing_names);
    for (key, expected) in source_attributes {
        let mut expected = expected;
        expected.sort_by(|left, right| left.1.cmp(&right.1));
        let mut actual = translated_attributes.get(&key).cloned().unwrap_or_default();
        actual.sort_by(|left, right| left.1.cmp(&right.1));
        let actual_values = actual
            .iter()
            .map(|(_, attributes)| attributes.clone())
            .collect::<Vec<_>>();
        let expected_values = expected
            .iter()
            .map(|(_, attributes)| attributes.clone())
            .collect::<Vec<_>>();
        if expected_values == actual_values {
            continue;
        }
        let mismatch = actual
            .iter()
            .find(|(_, attributes)| !expected_values.contains(attributes))
            .or_else(|| actual.first());
        if let Some((tag, attributes)) = mismatch {
            return Err(MarkupTranslationError::AttributeChange {
                tag: tag.name.clone(),
                expected: expected_values.first().cloned().unwrap_or_default(),
                actual: attributes.clone(),
                span: tag.span.clone(),
            });
        }
    }
    Ok(())
}

type StructuralTagKey = (MarkupTagKind, String, Vec<String>);

fn structural_attributes<'a>(
    tags: &'a [ParsedTag],
    closing_names: &BTreeSet<String>,
) -> BTreeMap<StructuralTagKey, Vec<(&'a ParsedTag, String)>> {
    let mut ancestors = Vec::new();
    let mut attributes = BTreeMap::new();
    for tag in tags {
        let key = (tag.kind, tag.name.clone(), ancestors.clone());
        attributes
            .entry(key)
            .or_insert_with(Vec::new)
            .push((tag, tag.attributes.clone()));
        match tag.kind {
            MarkupTagKind::Open if closing_names.contains(&tag.name) => {
                ancestors.push(tag.name.clone());
            }
            MarkupTagKind::Close if closing_names.contains(&tag.name) => {
                ancestors.pop();
            }
            MarkupTagKind::Open | MarkupTagKind::Close => {}
        }
    }
    attributes
}

fn validate_translation_balance(
    tags: &[ParsedTag],
    closing_names: &BTreeSet<String>,
) -> Result<(), MarkupTranslationError> {
    let mut open_tags = Vec::<&ParsedTag>::new();
    for tag in tags {
        match tag.kind {
            MarkupTagKind::Open if closing_names.contains(&tag.name) => open_tags.push(tag),
            MarkupTagKind::Open => {}
            MarkupTagKind::Close if !closing_names.contains(&tag.name) => {
                return Err(MarkupTranslationError::UnbalancedTag {
                    tag: tag.name.clone(),
                    span: tag.span.clone(),
                    kind: MarkupUnbalancedKind::Standalone,
                    related_opening: None,
                });
            }
            MarkupTagKind::Close => {
                let Some(opening) = open_tags.last() else {
                    return Err(MarkupTranslationError::UnbalancedTag {
                        tag: tag.name.clone(),
                        span: tag.span.clone(),
                        kind: MarkupUnbalancedKind::NoOpening,
                        related_opening: None,
                    });
                };
                if opening.name == tag.name {
                    open_tags.pop();
                } else {
                    return Err(MarkupTranslationError::UnbalancedTag {
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
    if let Some(opening) = open_tags.first() {
        return Err(MarkupTranslationError::MissingTag {
            tag: format!("/{}", opening.name),
        });
    }
    Ok(())
}

fn translation_error_from_issue(issue: MarkupValidationIssue) -> MarkupTranslationError {
    match issue {
        MarkupValidationIssue::UnbalancedTag {
            tag,
            span,
            kind,
            related_opening,
        } => MarkupTranslationError::UnbalancedTag {
            tag,
            span,
            kind,
            related_opening,
        },
        _ => MarkupTranslationError::UnbalancedTag {
            tag: "[".to_owned(),
            span: 0..1,
            kind: MarkupUnbalancedKind::MissingClosingBracket,
            related_opening: None,
        },
    }
}
