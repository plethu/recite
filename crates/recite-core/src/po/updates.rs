//! Atomic field and review changes validate the complete final source once.
use super::{PoDocument, PoEdit, PoEditError, PoEntry, PoEntryField, PoEntryId, PoUpdate, parser};
use std::{collections::BTreeMap, ops::Range};

#[derive(Default)]
struct EntryChanges {
    edits: Vec<PoEdit>,
    fuzzy: Option<bool>,
}

impl PoDocument {
    /// Apply fields and final fuzzy flags transactionally, preserving untouched bytes.
    ///
    /// All entry IDs and ranges refer to this document. Duplicate fields or fuzzy
    /// updates are rejected. Validation applies to the final source, so coordinated
    /// changes need not produce valid intermediate documents. Invalid unreviewed
    /// translations remain permitted by the ordinary fuzzy-entry rules.
    pub fn apply_updates(
        &mut self,
        updates: impl IntoIterator<Item = PoUpdate>,
    ) -> Result<(), PoEditError> {
        let mut changes: BTreeMap<PoEntryId, EntryChanges> = BTreeMap::new();
        for update in updates {
            let id = match &update {
                PoUpdate::Field(edit) => edit.entry,
                PoUpdate::Fuzzy { entry, .. } => *entry,
            };
            self.entry(id).ok_or(PoEditError::EntryNotFound(id))?;
            let change = changes.entry(id).or_default();
            match update {
                PoUpdate::Field(edit) => change.edits.push(edit),
                PoUpdate::Fuzzy { fuzzy, .. } => {
                    if change.fuzzy.replace(fuzzy).is_some() {
                        return Err(PoEditError::OverlappingEdits);
                    }
                }
            }
        }
        if changes.is_empty() {
            return Ok(());
        }
        let mut candidate = String::with_capacity(self.source.len());
        let mut offset = 0;
        for (id, change) in changes {
            let entry = self.entry(id).ok_or(PoEditError::EntryNotFound(id))?;
            candidate.push_str(&self.source[offset..entry.range.start]);
            candidate.push_str(&self.render_entry(entry, change)?);
            offset = entry.range.end;
        }
        candidate.push_str(&self.source[offset..]);
        let parsed = Self::parse_with_path(self.source_name.clone(), candidate)
            .map_err(PoEditError::InvalidDocument)?;
        *self = parsed;
        Ok(())
    }

    fn render_entry(&self, entry: &PoEntry, changes: EntryChanges) -> Result<String, PoEditError> {
        let mut replacements = Vec::with_capacity(changes.edits.len());
        for edit in changes.edits {
            replacements.push(self.field_replacement(entry, edit)?);
        }
        replacements.sort_by_key(|(range, _)| (range.start, range.end));
        if replacements
            .windows(2)
            .any(|pair| overlapping(&pair[0].0, &pair[1].0))
        {
            return Err(PoEditError::OverlappingEdits);
        }
        let mut rendered = String::with_capacity(entry.range.len());
        let mut offset = entry.range.start;
        for (range, replacement) in replacements {
            if range.start < offset {
                return Err(PoEditError::OverlappingEdits);
            }
            rendered.push_str(&self.source[offset..range.start]);
            rendered.push_str(&replacement);
            offset = range.end;
        }
        rendered.push_str(&self.source[offset..entry.range.end]);
        if let Some(fuzzy) = changes.fuzzy
            && entry.flags.iter().any(|flag| flag == "fuzzy") != fuzzy
        {
            rendered =
                super::review::format_fuzzy(&rendered, fuzzy, entry.obsolete, self.line_ending);
        }
        Ok(rendered)
    }

    fn field_replacement(
        &self,
        entry: &PoEntry,
        edit: PoEdit,
    ) -> Result<(Range<usize>, String), PoEditError> {
        let (range, keyword, multiline, obsolete) = entry.edit_range(&edit.field)?;
        let value = match &edit.field {
            PoEntryField::Variant => {
                let context = entry
                    .context
                    .as_deref()
                    .ok_or(PoEditError::MissingContext)?;
                let base = context.split('&').next().unwrap_or(context);
                if edit.value.is_empty() {
                    base.to_owned()
                } else if edit.value.contains('&') || edit.value.trim().is_empty() {
                    return Err(PoEditError::InvalidVariant(edit.value));
                } else {
                    format!("{base}&{}", edit.value)
                }
            }
            _ => edit.value,
        };
        let inserting_context = range.is_empty() && matches!(edit.field, PoEntryField::Context);
        let mut replacement = parser::format_field(
            &keyword,
            &value,
            multiline || value.contains(['\n', '\r']),
            self.line_ending,
            obsolete,
        );
        if inserting_context {
            replacement.push_str(self.line_ending);
        }
        Ok((range, replacement))
    }
}

fn overlapping(left: &Range<usize>, right: &Range<usize>) -> bool {
    if left.is_empty() || right.is_empty() {
        left.start == right.start
    } else {
        left.start < right.end && right.start < left.end
    }
}
