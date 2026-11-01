//! Bounded excerpts preserve the matcher's word or substring policy without retaining source.
use super::{SearchHit, words};
use std::collections::BTreeSet;

impl SearchHit {
    /// A bounded Unicode excerpt containing the first matching prose word.
    /// Matches in document, beat or speaker context fall back to the opening text.
    pub fn excerpt(&self, query: &str, limit: usize) -> String {
        let terms = words(query).collect::<BTreeSet<_>>();
        excerpt_at(
            &self.text,
            first_match(&self.text, &terms).unwrap_or(0),
            limit,
        )
    }
}

/// Match-centred passage lookup text, using case-insensitive whitespace-delimited substrings.
/// Saved index hits use their complete-word matcher through [`SearchHit::excerpt`].
pub fn passage_excerpt(text: &str, query: &str, limit: usize) -> String {
    let query = query.trim().to_lowercase();
    let terms = query.split_whitespace().collect::<Vec<_>>();
    excerpt_at(text, first_substring(text, &terms).unwrap_or(0), limit)
}

fn excerpt_at(text: &str, matched: usize, limit: usize) -> String {
    if limit < 4 {
        return text.chars().take(limit).collect();
    }
    let mut start = text[..matched]
        .char_indices()
        .rev()
        .nth(limit / 3)
        .map_or(0, |(offset, _)| offset);
    if start != 0
        && let Some(boundary) = text[start..matched].find(char::is_whitespace)
    {
        start += boundary;
    }
    let prefix = if start == 0 { "" } else { "… " };
    let available = limit - prefix.chars().count();
    let text = text[start..].trim_start();
    let end = text.char_indices().nth(available).map(|(offset, _)| offset);
    let mut result = String::from(prefix);
    match end {
        Some(_) => {
            result.extend(text.chars().take(available - 1));
            result.push('…');
        }
        None => result.push_str(text),
    }
    result
}

fn first_substring(text: &str, terms: &[&str]) -> Option<usize> {
    if terms.is_empty() {
        return None;
    }
    let mut offset = 0;
    for chunk in text.split_inclusive(char::is_whitespace) {
        let folded = chunk.to_lowercase();
        if let Some(matched) = terms.iter().filter_map(|term| folded.find(term)).min() {
            // Lowercasing can expand a character (İ → i + combining dot), so
            // a folded byte offset cannot index the original prose directly.
            let mut folded_end = 0;
            return chunk.char_indices().find_map(|(index, character)| {
                folded_end += character.to_lowercase().map(char::len_utf8).sum::<usize>();
                (matched < folded_end).then_some(offset + index)
            });
        }
        offset += chunk.len();
    }
    None
}

fn first_match(text: &str, terms: &BTreeSet<String>) -> Option<usize> {
    let mut offset = 0;
    for segment in text.split_inclusive(|c: char| !c.is_alphanumeric() && c != '_') {
        let start = offset;
        offset += segment.len();
        let word = segment.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_');
        if !word.is_empty() && terms.contains(&word.to_lowercase()) {
            return Some(start);
        }
    }
    None
}
