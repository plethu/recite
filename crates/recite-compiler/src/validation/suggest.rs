//! The closest known name to a misspelt one, for "did you mean" help.
//!
//! The choice is deterministic: the smallest edit distance within a bound that grows with the
//! name's length, ties broken by name order, and nothing when no candidate is close enough to be a
//! plausible typo.

/// The candidate closest to `name`, when one is plausibly a typo of it.
pub(crate) fn closest<'c>(
    name: &str,
    candidates: impl IntoIterator<Item = &'c str>,
) -> Option<&'c str> {
    let length = name.chars().count();
    let bound = (length / 3).max(1);
    candidates
        .into_iter()
        .filter(|candidate| *candidate != name)
        .map(|candidate| (edit_distance(name, candidate), candidate))
        .filter(|(distance, _)| *distance <= bound)
        .min()
        .map(|(_, candidate)| candidate)
}

/// Levenshtein distance over characters.
fn edit_distance(left: &str, right: &str) -> usize {
    let right = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    let mut current = vec![0; right.len() + 1];
    for (row, left_char) in left.chars().enumerate() {
        current[0] = row + 1;
        for (column, right_char) in right.iter().enumerate() {
            let substitution = previous[column] + usize::from(left_char != *right_char);
            current[column + 1] = substitution
                .min(previous[column + 1] + 1)
                .min(current[column] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

#[cfg(test)]
mod tests;
