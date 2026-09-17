//! Review state is the gettext fuzzy flag, not a second catalogue format.
use super::{PoDocument, PoEditError, PoEntryId};

impl PoDocument {
    /// Set an entry's gettext `fuzzy` flag, preserving other comments and flags.
    ///
    /// Clearing the flag revalidates the complete candidate. Invalid translations
    /// therefore remain drafts, and this document is unchanged on failure.
    pub fn set_fuzzy(&mut self, id: PoEntryId, fuzzy: bool) -> Result<(), PoEditError> {
        let entry = self.entry(id).ok_or(PoEditError::EntryNotFound(id))?;
        let existing = entry.flags.iter().any(|flag| flag == "fuzzy");
        if existing == fuzzy {
            return Ok(());
        }
        let range = entry.range.clone();
        let prefix = if entry.obsolete { "#~ ," } else { "#," };
        let mut replacement = String::new();
        if fuzzy {
            replacement.push_str(&format!("{prefix} fuzzy{}", self.line_ending));
            replacement.push_str(&self.source[range.clone()]);
        } else {
            for line in self.source[range.clone()].split_inclusive('\n') {
                let trimmed = line.trim_start();
                let flags = if entry.obsolete {
                    trimmed
                        .strip_prefix("#~")
                        .and_then(|s| s.trim_start().strip_prefix(','))
                } else {
                    trimmed
                        .strip_prefix('#')
                        .and_then(|s| s.trim_start().strip_prefix(','))
                };
                if let Some(flags) = flags {
                    let flags: Vec<_> = flags.trim().split(',').map(str::trim).collect();
                    if flags.contains(&"fuzzy") {
                        let retained: Vec<_> =
                            flags.into_iter().filter(|f| *f != "fuzzy").collect();
                        if !retained.is_empty() {
                            replacement.push_str(&format!(
                                "{prefix} {}{}",
                                retained.join(", "),
                                self.line_ending
                            ));
                        }
                        continue;
                    }
                }
                replacement.push_str(line);
            }
        }
        let mut candidate = self.source.clone();
        candidate.replace_range(range, &replacement);
        *self = Self::parse_with_path(self.source_name.clone(), candidate)
            .map_err(PoEditError::InvalidDocument)?;
        Ok(())
    }
}
