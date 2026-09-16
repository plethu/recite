//! UTF-8 replacement transactions; retained text has an explicit memory budget.
use std::collections::VecDeque;
const BUDGET: usize = 32 * 1024 * 1024;

pub(crate) struct Change {
    start: usize,
    removed: String,
    inserted: String,
}
impl Change {
    pub fn between(before: &str, after: &str) -> Self {
        let start = before
            .chars()
            .zip(after.chars())
            .take_while(|(a, b)| a == b)
            .map(|(c, _)| c.len_utf8())
            .sum();
        let suffix: usize = before[start..]
            .chars()
            .rev()
            .zip(after[start..].chars().rev())
            .take_while(|(a, b)| a == b)
            .map(|(c, _)| c.len_utf8())
            .sum();
        Self {
            start,
            removed: before[start..before.len() - suffix].into(),
            inserted: after[start..after.len() - suffix].into(),
        }
    }
    fn apply(&self, source: &str, undo: bool) -> String {
        let (old, new) = if undo {
            (&self.inserted, &self.removed)
        } else {
            (&self.removed, &self.inserted)
        };
        let mut source = source.to_owned();
        source.replace_range(self.start..self.start + old.len(), new);
        source
    }
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.removed.capacity() + self.inserted.capacity()
    }
}
#[derive(Default)]
pub(crate) struct History {
    undo: VecDeque<Change>,
    redo: Vec<Change>,
    bytes: usize,
}
impl History {
    pub fn record(&mut self, change: Change) {
        self.bytes -= self.redo.iter().map(Change::bytes).sum::<usize>();
        self.redo.clear();
        self.bytes += change.bytes();
        self.undo.push_back(change);
        // Always retain the latest transaction, including a whole-file replacement.
        while self.bytes > BUDGET && self.undo.len() > 1 {
            if let Some(old) = self.undo.pop_front() {
                self.bytes -= old.bytes();
            }
        }
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
    pub fn undo_source(&self, source: &str) -> Option<String> {
        self.undo.back().map(|c| c.apply(source, true))
    }
    pub fn redo_source(&self, source: &str) -> Option<String> {
        self.redo.last().map(|c| c.apply(source, false))
    }
    pub fn did_undo(&mut self) {
        if let Some(change) = self.undo.pop_back() {
            self.redo.push(change);
        }
    }
    pub fn did_redo(&mut self) {
        if let Some(change) = self.redo.pop() {
            self.undo.push_back(change);
        }
    }
}
