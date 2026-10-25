//! Bound emphasis work to visible snippets without splitting case-expanded Unicode characters.
use std::ops::Range;

pub(super) fn ranges(text: &str, query: &str) -> Vec<Range<usize>> {
    let mut folded = String::new();
    let mut origins = Vec::new();
    for (start, character) in text.char_indices() {
        let end = start + character.len_utf8();
        for lower in character.to_lowercase() {
            folded.push(lower);
            origins.extend(std::iter::repeat_n(start..end, lower.len_utf8()));
        }
    }
    // Emphasis is a bounded presentation aid; search ranking still uses the complete query.
    let mut ranges = query
        .split_whitespace()
        .take(16)
        .flat_map(|word| {
            let word = word.to_lowercase();
            folded
                .match_indices(&word)
                .map(|(start, _)| origins[start].start..origins[start + word.len() - 1].end)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    ranges.sort_by_key(|range| range.start);
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in ranges {
        if let Some(previous) = merged.last_mut()
            && range.start <= previous.end
        {
            previous.end = previous.end.max(range.end);
        } else {
            merged.push(range);
        }
    }
    merged
}
