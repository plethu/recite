//! Project word lists: one word per line, `#` starts a comment line.

use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::time::SystemTime;

use crate::{ProseEdit, ProseSetupError};

/// Words from every configured dictionary, deduplicated and sorted.
pub(crate) fn load_words(paths: &[impl AsRef<Path>]) -> Result<BTreeSet<String>, ProseSetupError> {
    let mut words = BTreeSet::new();
    for path in paths {
        let path = path.as_ref();
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(ProseSetupError::Dictionary {
                    path: path.display().to_string(),
                    detail: error.to_string(),
                });
            }
        };
        words.extend(parse_words(&text).map(ToOwned::to_owned));
    }
    Ok(words)
}

/// The words in one dictionary text, in file order.
pub fn parse_words(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

/// The edit that appends `word` to a dictionary whose current text is
/// `existing` (`None` for a file that does not exist yet). Returns `None`
/// when the word is already listed or cannot be stored on one line.
#[must_use]
pub fn dictionary_addition(existing: Option<&str>, word: &str) -> Option<ProseEdit> {
    let word = word.trim();
    if word.is_empty() || word.contains(['\n', '\r']) || word.starts_with('#') {
        return None;
    }
    let existing = existing.unwrap_or_default();
    if parse_words(existing).any(|listed| listed == word) {
        return None;
    }
    let separator = if existing.is_empty() || existing.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    Some(ProseEdit {
        range: existing.len()..existing.len(),
        replacement: format!("{separator}{word}\n"),
    })
}

/// Modification stamps of the dictionaries, used to notice edits.
pub(crate) fn stamps(paths: &[impl AsRef<Path>]) -> Vec<Option<(SystemTime, u64)>> {
    paths
        .iter()
        .map(|path| {
            fs::metadata(path)
                .ok()
                .and_then(|metadata| Some((metadata.modified().ok()?, metadata.len())))
        })
        .collect()
}
