//! The closest known name to a misspelt one, for "did you mean" help.
//!
//! The choice is deterministic: the smallest edit distance within a bound that grows with the
//! name's length, ties broken by name order, and nothing when no candidate is close enough to be a
//! plausible typo. A project can declare thousands of names, so candidates whose length alone rules
//! them out are skipped without allocating, and the rest share one set of buffers.

/// The candidate closest to `name`, when one is plausibly a typo of it.
pub(crate) fn closest<'c>(
    name: &str,
    candidates: impl IntoIterator<Item = &'c str>,
) -> Option<&'c str> {
    let length = name.chars().count();
    let bound = (length / 3).max(1);
    let mut distance = None;
    candidates
        .into_iter()
        .filter(|candidate| *candidate != name)
        .filter(|candidate| candidate.chars().count().abs_diff(length) <= bound)
        .map(|candidate| {
            let distance = distance.get_or_insert_with(|| Distance::to(name));
            (distance.from(candidate), candidate)
        })
        .filter(|(distance, _)| *distance <= bound)
        .min()
        .map(|(_, candidate)| candidate)
}

/// Levenshtein distance over characters to one target, reusing its rows between candidates.
struct Distance {
    target: Vec<char>,
    previous: Vec<usize>,
    current: Vec<usize>,
}

impl Distance {
    fn to(target: &str) -> Self {
        let target = target.chars().collect::<Vec<_>>();
        let width = target.len() + 1;
        Self {
            target,
            previous: Vec::with_capacity(width),
            current: vec![0; width],
        }
    }

    fn from(&mut self, source: &str) -> usize {
        self.previous.clear();
        self.previous.extend(0..=self.target.len());
        for (row, source_char) in source.chars().enumerate() {
            self.current[0] = row + 1;
            for (column, target_char) in self.target.iter().enumerate() {
                let substitution = self.previous[column] + usize::from(source_char != *target_char);
                self.current[column + 1] = substitution
                    .min(self.previous[column + 1] + 1)
                    .min(self.current[column] + 1);
            }
            std::mem::swap(&mut self.previous, &mut self.current);
        }
        self.previous[self.target.len()]
    }
}

#[cfg(test)]
mod tests;
