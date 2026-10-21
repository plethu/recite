//! One shared source-order budget bounds both reference and destination previews.
use recite_writer_model::ScriptEntry;
use std::slice;

pub struct ExcerptRow<'a> {
    pub entry: &'a ScriptEntry,
    pub depth: usize,
}

pub struct Excerpt<'a> {
    pending: Vec<slice::Iter<'a, ScriptEntry>>,
    remaining: usize,
}
impl<'a> Excerpt<'a> {
    pub fn new(entries: &'a [ScriptEntry]) -> Self {
        Self {
            pending: vec![entries.iter()],
            remaining: 32,
        }
    }
    pub fn limited(&self) -> bool {
        self.remaining == 0
            && self
                .pending
                .iter()
                .any(|entries| !entries.as_slice().is_empty())
    }
}
impl<'a> Iterator for Excerpt<'a> {
    type Item = ExcerptRow<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        loop {
            let entries = self.pending.last_mut()?;
            if let Some(entry) = entries.next() {
                self.remaining -= 1;
                let depth = self.pending.len() - 1;
                if let ScriptEntry::Group { entries, .. } = entry {
                    self.pending.push(entries.iter());
                }
                return Some(ExcerptRow { entry, depth });
            }
            self.pending.pop();
        }
    }
}
