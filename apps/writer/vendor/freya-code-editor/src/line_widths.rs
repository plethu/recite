//! Exact longest-line counts without a second positional index over the Rope.
use ropey::{Rope, RopeSlice};
use std::{collections::BTreeMap, ops::RangeInclusive};

#[derive(Default)]
pub(crate) struct LineWidths {
    counts: BTreeMap<usize, usize>,
    source: Option<Rope>,
    non_lf_breaks: usize,
    #[cfg(test)]
    pub(crate) visited: usize,
}

impl LineWidths {
    fn ensure(&mut self, rope: &Rope) {
        if self
            .source
            .as_ref()
            .is_some_and(|old| old.is_instance(rope))
        {
            return;
        }
        self.counts.clear();
        self.non_lf_breaks = 0;
        for line in rope.lines() {
            #[cfg(test)]
            {
                self.visited += 1;
            }
            *self.counts.entry(line.len_chars()).or_default() += 1;
            self.non_lf_breaks += usize::from(non_lf(line));
        }
        self.source = Some(rope.clone());
    }

    pub(crate) fn has_non_lf_breaks(&mut self, rope: &Rope) -> bool {
        self.ensure(rope);
        self.non_lf_breaks != 0
    }

    #[cfg(test)]
    pub(crate) fn assert_matches(&mut self, rope: &Rope) {
        self.ensure(rope);
        let mut fresh = Self::default();
        fresh.ensure(rope);
        assert_eq!(self.counts, fresh.counts);
        assert_eq!(self.non_lf_breaks, fresh.non_lf_breaks);
    }

    pub(crate) fn longest(&mut self, rope: &Rope) -> usize {
        self.ensure(rope);
        self.counts
            .last_key_value()
            .map_or(0, |(&length, _)| length)
    }

    pub(crate) fn remove(&mut self, rope: &Rope, rows: RangeInclusive<usize>) {
        self.ensure(rope);
        for row in rows {
            #[cfg(test)]
            {
                self.visited += 1;
            }
            let line = rope.line(row);
            self.non_lf_breaks -= usize::from(non_lf(line));
            let length = line.len_chars();
            if let Some(count) = self.counts.get_mut(&length) {
                *count -= 1;
                if *count == 0 {
                    self.counts.remove(&length);
                }
            }
        }
    }

    pub(crate) fn insert(&mut self, rope: &Rope, rows: RangeInclusive<usize>) {
        for row in rows {
            #[cfg(test)]
            {
                self.visited += 1;
            }
            let line = rope.line(row);
            self.non_lf_breaks += usize::from(non_lf(line));
            *self.counts.entry(line.len_chars()).or_default() += 1;
        }
        self.source = Some(rope.clone());
    }
}

fn non_lf(line: RopeSlice<'_>) -> bool {
    line.len_chars().checked_sub(1).is_some_and(|last| {
        matches!(
            line.char(last),
            '\r' | '\u{b}' | '\u{c}' | '\u{85}' | '\u{2028}' | '\u{2029}'
        )
    })
}
