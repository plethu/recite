//! A fixed native-field budget with overlap between explicit reading steps.
use std::ops::Range;

pub const CAPACITY: usize = 16;
pub const STEP: usize = CAPACITY / 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Previous,
    Next,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReadingWindow {
    start: usize,
    shifted: bool,
}
impl ReadingWindow {
    pub fn range(self, total: usize) -> Range<usize> {
        let start = self.start.min(total.saturating_sub(1));
        start..start.saturating_add(CAPACITY).min(total)
    }
    pub fn start(self) -> usize {
        self.start
    }
    pub fn normalize(&mut self, total: usize) {
        if self.start >= total {
            self.start = total.saturating_sub(1) / CAPACITY * CAPACITY;
            self.shifted = false;
        }
    }
    pub fn is_shift(self) -> bool {
        self.shifted
    }
    pub fn reveal(&mut self, index: usize) {
        if !(self.start..self.start.saturating_add(CAPACITY)).contains(&index) {
            self.start = index / CAPACITY * CAPACITY;
            self.shifted = false;
        }
    }
    pub fn shifted(self, direction: Direction, total: usize) -> Option<Self> {
        let range = self.range(total);
        let start = match direction {
            Direction::Previous if range.start > 0 => range.start.saturating_sub(STEP),
            Direction::Next if range.end < total => range.start.saturating_add(STEP),
            _ => return None,
        };
        Some(Self {
            start,
            shifted: true,
        })
    }
}

#[cfg(test)]
mod tests;
