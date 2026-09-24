//! Refresh by exact context identity; never guess translation similarity.
use super::{PoDocument, PoEntry, PoEntryField, PoParseError, parser};
use std::collections::{BTreeMap, BTreeSet};

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum PoRefreshError {
    #[error("catalogue refresh requires unique nonempty contexts: {0}")]
    AmbiguousContext(String),
    #[error("plural shape changed for {0}; migrate this entry explicitly")]
    PluralShape(String),
    #[error("new plural entries require valid target-language Plural-Forms metadata")]
    MissingPluralRule,
    #[error("refreshed catalogue is invalid: {0}")]
    InvalidDocument(#[from] PoParseError),
}

impl PoDocument {
    /// Refresh against a complete extracted template, retaining translations and
    /// unknown fields. Changed source is fuzzy; removed entries become obsolete.
    /// Variant contexts follow their base context when absent from the template.
    /// Headers and existing obsolete entries are preserved. No files are written.
    /// Ambiguous contexts and singular/plural shape changes fail without mutation.
    pub fn refreshed(&self, template: &Self) -> Result<Self, PoRefreshError> {
        let targets = contexts(template)?;
        contexts(self)?;
        let mut matched = BTreeSet::new();
        let mut replacements = Vec::new();
        for entry in self
            .entries()
            .iter()
            .filter(|e| !e.is_header() && !e.is_obsolete())
        {
            let context = entry.context().unwrap_or_default();
            let target = targets.get(context).or_else(|| {
                context
                    .split_once('&')
                    .and_then(|(base, _)| targets.get(base))
            });
            let raw = &self.source[entry.range.clone()];
            let replacement = if let Some(target) = target {
                if entry.context() == target.context() {
                    matched.insert(target.context().unwrap_or_default());
                }
                if entry.is_plural() != target.is_plural() {
                    return Err(PoRefreshError::PluralShape(context.into()));
                }
                refresh_entry(self, entry, target)
            } else {
                raw.split_inclusive('\n')
                    .map(|line| {
                        if line.trim().is_empty() {
                            line.to_owned()
                        } else {
                            format!("#~ {line}")
                        }
                    })
                    .collect()
            };
            replacements.push((entry.range.clone(), replacement));
        }
        let mut source = String::with_capacity(self.source.len());
        let mut cursor = 0;
        for (range, replacement) in replacements {
            source.push_str(&self.source[cursor..range.start]);
            source.push_str(&replacement);
            cursor = range.end;
        }
        source.push_str(&self.source[cursor..]);
        for entry in template
            .entries()
            .iter()
            .filter(|e| !e.is_header() && !e.is_obsolete())
        {
            if matched.contains(entry.context().unwrap_or_default()) {
                continue;
            }
            // A variant does not satisfy the base entry: retain a default translation too.
            source.push_str(self.line_ending);
            source.push_str(self.line_ending);
            source.push_str(&new_entry(self, entry)?);
        }
        Ok(Self::parse_with_path(self.source_name.clone(), source)?)
    }
}

fn contexts(document: &PoDocument) -> Result<BTreeMap<&str, &PoEntry>, PoRefreshError> {
    let mut entries = BTreeMap::new();
    for entry in document
        .entries()
        .iter()
        .filter(|e| !e.is_header() && !e.is_obsolete())
    {
        let context = entry
            .context()
            .filter(|c| !c.is_empty())
            .ok_or_else(|| PoRefreshError::AmbiguousContext(String::new()))?;
        if entries.insert(context, entry).is_some() {
            return Err(PoRefreshError::AmbiguousContext(context.into()));
        }
    }
    Ok(entries)
}

fn metadata(entry: &PoEntry, ending: &str) -> String {
    entry
        .comments()
        .iter()
        .filter_map(|c| {
            let prefix = match c.kind() {
                super::PoCommentKind::Extracted => "#.",
                super::PoCommentKind::Reference => "#:",
                _ => return None,
            };
            Some(format!("{prefix} {}{ending}", c.text()))
        })
        .collect()
}

fn refresh_entry(document: &PoDocument, entry: &PoEntry, target: &PoEntry) -> String {
    let mut raw = document.source[entry.range.clone()].to_owned();
    let changed = entry.source_text() != target.source_text()
        || entry.plural_source_text() != target.plural_source_text();
    let mut edits = Vec::new();
    for (field, value) in [
        (PoEntryField::SourceText, Some(target.source_text())),
        (PoEntryField::PluralSourceText, target.plural_source_text()),
    ] {
        let existing = match field {
            PoEntryField::SourceText => Some(entry.source_text()),
            PoEntryField::PluralSourceText => entry.plural_source_text(),
            _ => None,
        };
        if value != existing
            && let Some(value) = value
            && let Ok((range, keyword, multiline, _)) = entry.edit_range(&field)
        {
            edits.push((
                range,
                parser::format_field(&keyword, value, multiline, document.line_ending, false),
            ));
        }
    }
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    for (range, text) in edits {
        raw.replace_range(
            range.start - entry.range.start..range.end - entry.range.start,
            &text,
        );
    }
    // Only extractor-owned comments are replaced. Translator notes and vendor
    // fields, flags, previous values and formatting otherwise remain intact.
    let retained: String = raw
        .split_inclusive('\n')
        .filter(|line| {
            let line = line.trim_start();
            !line.starts_with("#.") && !line.starts_with("#:")
        })
        .collect();
    let fuzzy = if changed && !entry.flags().iter().any(|f| f == "fuzzy") {
        format!("#, fuzzy{}", document.line_ending)
    } else {
        String::new()
    };
    let mut previous = String::new();
    if changed && entry.previous().is_empty() {
        for (key, value) in [
            ("msgid", Some(entry.source_text())),
            ("msgid_plural", entry.plural_source_text()),
        ] {
            if let Some(value) = value {
                previous.push_str("#| ");
                previous.push_str(&parser::format_field(
                    key,
                    value,
                    false,
                    document.line_ending,
                    false,
                ));
                previous.push_str(document.line_ending);
            }
        }
    }
    format!(
        "{}{fuzzy}{previous}{retained}",
        metadata(target, document.line_ending)
    )
}

fn new_entry(document: &PoDocument, entry: &PoEntry) -> Result<String, PoRefreshError> {
    let ending = document.line_ending;
    let mut output = metadata(entry, ending);
    for (key, value) in [
        ("msgctxt", entry.context()),
        ("msgid", Some(entry.source_text())),
        ("msgid_plural", entry.plural_source_text()),
    ] {
        if let Some(value) = value {
            output.push_str(&parser::format_field(key, value, false, ending, false));
            output.push_str(ending);
        }
    }
    if entry.is_plural() {
        let count = document
            .headers()
            .iter()
            .find(|h| h.key().eq_ignore_ascii_case("Plural-Forms"))
            .and_then(|h| super::validate_plural_rule(h.value()).ok())
            .ok_or(PoRefreshError::MissingPluralRule)?;
        for index in 0..count {
            output.push_str(&format!("msgstr[{index}] \"\"{ending}"));
        }
    } else {
        output.push_str(&format!("msgstr \"\"{ending}"));
    }
    Ok(output)
}
