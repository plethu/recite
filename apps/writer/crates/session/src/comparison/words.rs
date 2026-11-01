//! Bound emphasis at complete UTF-8 words without changing the compared text.
pub fn changed_range(text: &str, other: &str) -> std::ops::Range<usize> {
    let words: Vec<_> = text.split_inclusive(char::is_whitespace).collect();
    let other: Vec<_> = other.split_inclusive(char::is_whitespace).collect();
    let prefix = words.iter().zip(&other).take_while(|(a, b)| a == b).count();
    let suffix = words[prefix..]
        .iter()
        .rev()
        .zip(other[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    words[..prefix].iter().map(|s| s.len()).sum()
        ..words[..words.len() - suffix].iter().map(|s| s.len()).sum()
}

#[cfg(test)]
mod tests;
