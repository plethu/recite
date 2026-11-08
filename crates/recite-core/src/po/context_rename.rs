//! Source-preserving rewrites of the stable ID that keys PO contexts.
//!
//! Dialogue catalogues key each line or choice on `msgctxt`, which holds the
//! bare stable ID or `id&variant`. Renaming an ID rewrites every context keyed
//! on it, including obsolete (`#~`) records and previous (`#|`) values, so
//! revived entries and fuzzy review history stay attached to the new ID.

use std::ops::Range;

use super::parser::{self, PoFieldTarget, PoPreviousField};
use super::{PoDocument, PoEditError};

/// One replacement of original document bytes planned against a PO source.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PoSourceReplacement {
    range: Range<usize>,
    text: String,
}

impl PoSourceReplacement {
    /// Byte range in the planned document's source.
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }

    /// Replacement source text, including PO quoting and comment prefixes.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

impl PoDocument {
    /// Whether any active, obsolete or previous context is keyed on `id`,
    /// either bare or with a `&variant` suffix.
    #[must_use]
    pub fn has_context_key(&self, id: &str) -> bool {
        self.entries.iter().any(|entry| {
            entry
                .context
                .as_deref()
                .is_some_and(|context| renamed_context(context, id, id).is_some())
                || entry.previous.iter().any(|previous| {
                    *previous.field() == PoPreviousField::Context
                        && renamed_context(previous.value(), id, id).is_some()
                })
        })
    }

    /// Plans replacing the stable ID `from` with `to` in every context keyed
    /// on it, preserving variant suffixes and all untouched bytes.
    ///
    /// Replacements are ordered by range. The plan is checked by parsing the
    /// complete final source, so applying it cannot produce an invalid PO
    /// document. An empty plan means no context uses `from`.
    pub fn plan_context_rename(
        &self,
        from: &str,
        to: &str,
    ) -> Result<Vec<PoSourceReplacement>, PoEditError> {
        let mut replacements = Vec::new();
        for entry in &self.entries {
            for field in &entry.fields {
                let previous = match field.target {
                    PoFieldTarget::Context => false,
                    PoFieldTarget::Previous(PoPreviousField::Context) => true,
                    _ => continue,
                };
                let value = if previous {
                    entry
                        .previous
                        .iter()
                        .find(|value| *value.field() == PoPreviousField::Context)
                        .map(super::PoPreviousValue::value)
                } else {
                    entry.context.as_deref()
                };
                let Some(renamed) = value.and_then(|value| renamed_context(value, from, to)) else {
                    continue;
                };
                let text = if previous {
                    format_previous(&field.keyword, &renamed, field, self.line_ending)
                } else {
                    parser::format_field(
                        &field.keyword,
                        &renamed,
                        field.multiline,
                        self.line_ending,
                        field.obsolete,
                    )
                };
                replacements.push(PoSourceReplacement {
                    range: field.range.clone(),
                    text,
                });
            }
        }
        replacements.sort_by_key(|replacement| (replacement.range.start, replacement.range.end));
        self.check_replacements(&replacements)?;
        Ok(replacements)
    }

    fn check_replacements(&self, replacements: &[PoSourceReplacement]) -> Result<(), PoEditError> {
        if replacements.is_empty() {
            return Ok(());
        }
        let mut candidate = String::with_capacity(self.source.len());
        let mut offset = 0;
        for replacement in replacements {
            if replacement.range.start < offset {
                return Err(PoEditError::OverlappingEdits);
            }
            candidate.push_str(&self.source[offset..replacement.range.start]);
            candidate.push_str(&replacement.text);
            offset = replacement.range.end;
        }
        candidate.push_str(&self.source[offset..]);
        Self::parse_with_path(self.source_name.clone(), candidate)
            .map(drop)
            .map_err(PoEditError::InvalidDocument)
    }
}

/// The context with its stable-ID prefix replaced, when keyed on `from`.
fn renamed_context(context: &str, from: &str, to: &str) -> Option<String> {
    if context == from {
        return Some(to.to_owned());
    }
    let variant = context.strip_prefix(from)?.strip_prefix('&')?;
    Some(format!("{to}&{variant}"))
}

fn format_previous(
    keyword: &str,
    value: &str,
    field: &parser::PoFieldRange,
    ending: &str,
) -> String {
    let prefix = if field.obsolete { "#~| " } else { "#| " };
    let value = super::escape::escape(value);
    if field.multiline {
        format!("{prefix}{keyword} \"\"{ending}{prefix}\"{value}\"")
    } else {
        format!("{prefix}{keyword} \"{value}\"")
    }
}
