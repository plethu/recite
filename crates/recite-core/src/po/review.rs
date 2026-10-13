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
        self.apply_updates([super::PoUpdate::Fuzzy { entry: id, fuzzy }])
    }
}

pub(super) fn format_fuzzy(source: &str, fuzzy: bool, obsolete: bool, line_ending: &str) -> String {
    let prefix = if obsolete { "#~ ," } else { "#," };
    let mut replacement = String::new();
    if fuzzy {
        replacement.push_str(&format!("{prefix} fuzzy{}", line_ending));
        replacement.push_str(source);
    } else {
        for line in source.split_inclusive('\n') {
            let trimmed = line.trim_start();
            let flags = if obsolete {
                trimmed
                    .strip_prefix("#~")
                    .and_then(|s| s.trim_start().strip_prefix(','))
            } else {
                trimmed
                    .strip_prefix('#')
                    .and_then(|s| s.trim_start().strip_prefix(','))
            };
            let Some(flags) = flags else {
                replacement.push_str(line);
                continue;
            };
            let flags: Vec<_> = flags.trim().split(',').map(str::trim).collect();
            if !flags.contains(&"fuzzy") {
                replacement.push_str(line);
                continue;
            }
            let retained: Vec<_> = flags.into_iter().filter(|f| *f != "fuzzy").collect();
            if !retained.is_empty() {
                replacement.push_str(&format!("{prefix} {}{}", retained.join(", "), line_ending));
            }
        }
    }
    replacement
}
